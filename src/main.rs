mod session_id;
use session_id::get_session_id;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::spawn;
use rtsp_types::{Message, ParseError, Request};


#[tokio::main]
async fn main() {
	let addr = "127.0.0.1:5554";
	let server = TcpListener::bind(addr).await.unwrap();

	loop {
		let (sock, addr) = server.accept().await.unwrap();
		println!("Got connection from {:?}", addr);

		spawn(async move {
			let mut intermediate_buffer = Vec::<u8>::with_capacity(512);
			let (reader, mut writer) = sock.into_split();
			let mut reader = BufReader::new(reader);
			loop {
				match reader.read_buf(&mut intermediate_buffer).await {
					Ok(bytes) => {
						if bytes > 0 {
							return match Message::<Vec<u8>>::parse(intermediate_buffer.as_slice()) {
								Ok((Message::Request(message), _)) => {
									if let Err(err) = connect_and_redirect(message, &mut writer).await {
										eprintln!("Encountered error: {:?}", err);
									}
									intermediate_buffer.clear();
									// break;
								},
								Ok((Message::Response(_), _)) => break,
								Ok((Message::Data(_), _)) => break,
								// If we don't have a full message yet, don't do anything
								Err(ParseError::Incomplete) => {},
								Err(ParseError::Error) => break,
							};
						}
					},
					Err(err) => {
						eprintln!("Could not read packet: {:?}", err);
						break;
					}
				}
			}
		});
	}
}

async fn connect_and_redirect(message: Request<Vec<u8>>, writer: &mut OwnedWriteHalf) -> anyhow::Result<()> {
	if let (Some(cseq), Some(uri)) = (message.header(&rtsp_types::headers::CSEQ), message.request_uri()) {
		if let Some(mut segments) = uri.path_segments() {
			if let (Some(host), Some(username), Some(password)) = (segments.next(), segments.next(), segments.next()) {
				if let Some(response) = write_redirect(cseq, host, username, password).await {
					// Respond
					let mut buffer = Vec::<u8>::new();
					response.write(&mut buffer)?;
					writer.write_all(&buffer).await?;
					writer.shutdown().await?;
				}
			}
		}
	}
	return Ok(());
}

async fn write_redirect(cseq: &rtsp_types::HeaderValue, host: &str, username: &str, password: &str) -> Option<rtsp_types::Response<rtsp_types::Empty>> {
	if let Ok(session_id) = get_session_id(host, username, password).await {
		let redirect_msg = rtsp_types::Response::builder(
			rtsp_types::Version::V2_0,
			rtsp_types::StatusCode::Found,
		)
			.header(rtsp_types::headers::CSEQ, cseq.as_str())
			.header(rtsp_types::headers::LOCATION, format!(
				"rtsp://{}/StreamingSetting?version=1.0&action=getRTSPStream&sessionID={}&ChannelID=1&ChannelName=Channel1",
				host,
				session_id,
			))
			.empty();

		return Some(redirect_msg);
	} else {
		return None;
	}
}
