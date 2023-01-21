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

// enum RoutedMessage {
// 	ToClient(Message<Vec<u8>>),
// 	ToServer(Message<Vec<u8>>),
// 	NoAction,
// 	Reset,
// }

// struct ProxySession {
// 	host: Option<String>,
// 	username: Option<String>,
// 	password: Option<String>,
// 	session_token: Option<String>,
// 	intermediate_buffer: Vec<u8>,
// }
// impl ProxySession {
// 	fn new() -> Self {
// 		return ProxySession {
// 			host: None,
// 			username: None,
// 			password: None,
// 			session_token: None,
// 			intermediate_buffer: Vec::with_capacity(8192),
// 		};
// 	}

// 	async fn handle_from_client(&mut self, message: &[u8]) -> RoutedMessage {
// 		self.intermediate_buffer.extend_from_slice(message);
// 		return match Message::<Vec<u8>>::parse(self.intermediate_buffer.as_slice()) {
// 			Ok((message, _)) => respond(self.process_client_request(message).await),
// 			Err(ParseError::Incomplete) => RoutedMessage::NoAction, // If we don't have a full message yet, don't do anything
// 			Err(ParseError::Error) => RoutedMessage::Reset,
// 		};
// 	}

// 	async fn rewrite_path(&mut self, req: &mut Request<Vec<u8>>) -> anyhow::Result<()> {
// 		if let Some(ref mut url) = req.request_uri() {
// 			println!("Received request for {}", url);
// 			let mut path_segments = url.path_segments().context("Could not get path segments from request")?;
// 			let host = path_segments.next().context("Could not get host")?;
// 			let username = path_segments.next();
// 			let password = path_segments.next();
// 			let session_id = self.get_session_id(Some(host), username, password).await.unwrap();
// 			req.set_request_uri(Some(Url::parse(&format!(
// 				"rtsp://{}/StreamingSetting?version=1.0&action=getRTSPStream&sessionID={}&ChannelID=1&ChannelName=Channel1",
// 				host,
// 				session_id,
// 			)).unwrap()));
// 			println!("Rewrote request: {}", req.request_uri().unwrap());
// 		}
// 		return Ok(());
// 	}

// 	async fn get_session_id<'a>(
// 		&'a mut self,
// 		host: Option<&str>,
// 		username: Option<&str>,
// 		password: Option<&str>,
// 	) -> anyhow::Result<&'a str> {
// 		if let Some(ref session_id) = self.session_token {
// 			return Ok(session_id);
// 		} else {
// 			let host = host.context("Missing host path segment in request")?;
// 			let username = username.context("Missing host path segment in request")?;
// 			let password = password.context("Missing host path segment in request")?;
// 			let session_id = get_session_id(&host, &username, &password).await?;
// 			self.session_token = Some(session_id);
// 			return Ok(self.session_token.as_ref().unwrap());
// 		}
// 	}

// 	/// Processes a client request and rewrites it in-transit on its way to the server
// 	async fn process_client_request(
// 		&mut self,
// 		mut message: Message<Vec<u8>>,
// 	) -> anyhow::Result<RoutedMessage> {
// 		match message {
// 			Message::Request(ref mut req) => {
// 				self.rewrite_path(req).await.context("Failed to rewrite the request path")?;
// 			},
// 			Message::Response(_) => { Err(anyhow!("The client sent an uninitiated response."))? },
// 			Message::Data(_) => { panic!("Can't handle data messages yet") },
// 		}
// 		self.intermediate_buffer.clear();
// 		return Ok(RoutedMessage::ToServer(message));
// 	}

// }

// fn respond(result: anyhow::Result<RoutedMessage>) -> RoutedMessage {
// 	return match result {
// 		Ok(message) => message,
// 		Err(err) => {
// 			eprintln!("Encountered an error. Resetting the connection. Error: {:?}", err);
// 			RoutedMessage::Reset
// 		},
// 	};
// }
