use tauri::{AppHandle, State};

use crate::{
    models::{
        AppBootstrap, ConnectServerProfileRequest, ConnectServerProfileResult, ProfileIdRequest,
    },
    ActiveSessionState,
};

use super::common::service;

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_app_state(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
) -> Result<AppBootstrap, String> {
    service(&app)?.bootstrap_app_state(&state.0).await
}

#[tauri::command]
#[specta::specta]
pub async fn connect_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: ConnectServerProfileRequest,
) -> Result<ConnectServerProfileResult, String> {
    service(&app)?
        .connect_server_profile(&state.0, payload)
        .await
}

#[tauri::command]
#[specta::specta]
pub fn delete_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: ProfileIdRequest,
) -> Result<AppBootstrap, String> {
    service(&app)?.delete_server_profile(&state.0, &payload.profile_id)
}
