use std::sync::Mutex;

use opensubsonic_client::{ApiError, OpenSubsonicClient};
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

pub(crate) fn trim_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn normalize_text(value: Option<&str>) -> Option<String> {
    value.and_then(|v| trim_text(v))
}

pub(crate) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|v| trim_text(&v))
}

pub(crate) fn normalize_media_type(media_type: Option<String>) -> Option<String> {
    media_type.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_ascii_lowercase())
    })
}

pub(crate) fn normalize_roles(roles: Vec<String>) -> Vec<String> {
    roles.into_iter().filter_map(|role| trim_text(&role)).collect()
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
