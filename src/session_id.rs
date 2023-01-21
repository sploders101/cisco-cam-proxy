use reqwest::header::ToStrError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GetSessionIDError {
	#[error("Failed to authenticate with camera")]
	AuthenticationFailed,
	#[error("An error occurred while making the authentication request.\n\n{0}")]
	RequestError(#[from] reqwest::Error),
	#[error("An error occurred while decoding the session ID")]
	ToStrError(#[from] ToStrError),
}

pub async fn get_session_id(ip_address: &str, username: &str, password: &str) -> Result<String, GetSessionIDError> {
	let client = reqwest::Client::builder()
		.danger_accept_invalid_certs(true)
		.build()?;
	// The cameras don't allow special characters in their passwords anyway, so we don't really need encoding here.
	// Worst-case scenario, a couple extra parameters are injected into the URL and the request is denied for a bad password.
	// This should only be used on a LAN anyway.
	let response = client.get(&format!("https://{ip_address}/System.xml?version=1.0&action=login&userName={username}&password={password}&sesionTemp=")).send().await?;
	let headers = response.headers();
	let session_id = if let Some(header) = headers.get("sessionID") { header } else { return Err(GetSessionIDError::AuthenticationFailed) };

	return Ok(String::from(session_id.to_str()?));
}
