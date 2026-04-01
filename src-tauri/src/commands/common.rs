use std::sync::Mutex;

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use opensubsonic_client::{ApiError, OpenSubsonicClient, PreparedBinaryRequest};
use tauri::{AppHandle, Manager};

use crate::{
    connection::ConnectionService,
    models::ActiveSession,
    secrets::{create_secret_store, AppSecretStore},
    session::SessionService,
};

type AppService = SessionService<AppSecretStore, ConnectionService>;

pub(crate) fn service(app: &AppHandle) -> Result<AppService, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app config directory: {error}"))?;
    let secret_service_name = format!("{}.server-profile", app.config().identifier);
    let secret_store = create_secret_store(&config_dir);
    let api = ConnectionService::new("transonic");

    Ok(SessionService::new(
        config_dir,
        secret_service_name,
        secret_store,
        api,
    ))
}

pub(crate) fn client(
    app: &AppHandle,
    sessions: &Mutex<Option<ActiveSession>>,
) -> Result<OpenSubsonicClient, String> {
    service(app)?.build_active_client(sessions)
}

pub(crate) fn format_api_error(error: ApiError) -> String {
    match error {
        ApiError::InvalidUrl(message)
        | ApiError::ClientBuild(message)
        | ApiError::Protocol(message) => message,
        ApiError::Transport(error) => error.to_string(),
        ApiError::HttpStatus { status, .. } => format!("HTTP {}", status.as_u16()),
        ApiError::Decode { message, .. } => message,
        ApiError::Api { code, message, .. } => {
            let detail = message.unwrap_or_else(|| "Unknown API error".to_string());
            format!("API {code}: {detail}")
        }
        ApiError::UnsupportedExtension { extension } => {
            format!("Missing extension: {extension}")
        }
    }
}

pub(crate) async fn fetch_binary_response(
    request: PreparedBinaryRequest,
) -> Result<(String, Vec<u8>), String> {
    let response = reqwest::Client::new()
        .get(request.url)
        .headers(request.headers)
        .send()
        .await
        .map_err(|error| format!("Failed to fetch binary data: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "The server returned HTTP {} for the binary request.",
            status.as_u16()
        ));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    if !content_type.starts_with("image/") {
        return Err(format!(
            "Unexpected binary content type for cover art: {content_type}"
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Failed to read binary response bytes: {error}"))?
        .to_vec();
    Ok((content_type, bytes))
}

pub(crate) fn encode_data_url(content_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    )
}
