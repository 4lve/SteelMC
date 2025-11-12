use reqwest::StatusCode;
use steel_core::player::GameProfile;
use thiserror::Error;

const MOJANG_AUTH_URL: &str =
    "https://sessionserver.mojang.com/session/minecraft/hasJoined?username=";
const SERVER_ID_ARG: &str = "&serverId=";

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Authentication servers are down")]
    FailedResponse,
    #[error("Failed to verify username")]
    UnverifiedUsername,
    #[error("You are banned from Authentication servers")]
    Banned,
    #[error("Texture Error {0}")]
    TextureError(TextureError),
    #[error("You have disallowed actions from Authentication servers")]
    DisallowedAction,
    #[error("Failed to parse JSON into Game Profile")]
    FailedParse,
    #[error("Unknown Status Code {0}")]
    UnknownStatusCode(StatusCode),
}

#[derive(Error, Debug)]
pub enum TextureError {
    #[error("Invalid URL")]
    InvalidURL,
    #[error("Invalid URL scheme for player texture: {0}")]
    DisallowedUrlScheme(String),
    #[error("Invalid URL domain for player texture: {0}")]
    DisallowedUrlDomain(String),
    #[error("Failed to decode base64 player texture: {0}")]
    DecodeError(String),
    #[error("Failed to parse JSON from player texture: {0}")]
    JSONError(String),
}

pub async fn mojang_authenticate(
    username: &str,
    server_hash: &str,
) -> Result<GameProfile, AuthError> {
    let cap = MOJANG_AUTH_URL.len() + SERVER_ID_ARG.len() + username.len() + server_hash.len();
    let mut auth_url = String::with_capacity(cap);
    auth_url += MOJANG_AUTH_URL;
    auth_url += username;
    auth_url += SERVER_ID_ARG;
    auth_url += server_hash;

    let response = reqwest::get(auth_url)
        .await
        .map_err(|_| AuthError::FailedResponse)?;

    match response.status() {
        StatusCode::OK => {}
        StatusCode::NO_CONTENT => Err(AuthError::UnverifiedUsername)?,
        other => Err(AuthError::UnknownStatusCode(other))?,
    }

    response.json().await.map_err(|_| AuthError::FailedParse)
}
