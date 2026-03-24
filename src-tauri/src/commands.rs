use tauri::{AppHandle, Manager, Runtime, State};

use crate::{
    connection::ConnectionService,
    models::{
        AppBootstrap, ConnectServerProfileRequest, ConnectServerProfileResult, ProfileIdRequest,
    },
    secrets::OsKeyringSecretStore,
    session::SessionService,
    ActiveSessionState,
};

#[tauri::command]
pub async fn bootstrap_app_state<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.bootstrap_app_state(&state.0).await
}

#[tauri::command]
pub async fn connect_server_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: ConnectServerProfileRequest,
) -> Result<ConnectServerProfileResult, String> {
    build_service(&app)?
        .connect_server_profile(&state.0, payload)
        .await
}

#[tauri::command]
pub fn delete_server_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: ProfileIdRequest,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.delete_server_profile(&state.0, &payload.profile_id)
}

fn build_service<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<SessionService<OsKeyringSecretStore, ConnectionService>, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app config directory: {error}"))?;
    let secret_service_name = format!("{}.server-profile", app.config().identifier);
    let api = ConnectionService::new("transonic");

    Ok(SessionService::new(
        config_dir,
        secret_service_name,
        OsKeyringSecretStore,
        api,
    ))
}
