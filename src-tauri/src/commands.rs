use tauri::{AppHandle, Manager, Runtime, State};

use crate::{
    models::{
        AppBootstrap, ConnectServerProfileRequest, ConnectServerProfileResult, ProfileIdRequest,
    },
    secrets::OsKeyringSecretStore,
    session::SessionService,
    subsonic_client::ReqwestSubsonicClient,
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
pub async fn activate_server_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: ProfileIdRequest,
) -> Result<ConnectServerProfileResult, String> {
    build_service(&app)?
        .activate_server_profile(&state.0, &payload.profile_id)
        .await
}

#[tauri::command]
pub fn disconnect_active_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.disconnect_active_profile(&state.0)
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
) -> Result<SessionService<OsKeyringSecretStore, ReqwestSubsonicClient>, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app config directory: {error}"))?;
    let secret_service_name = format!("{}.server-profile", app.config().identifier);
    let api = ReqwestSubsonicClient::new()?;

    Ok(SessionService::new(
        config_dir,
        secret_service_name,
        OsKeyringSecretStore,
        api,
    ))
}
