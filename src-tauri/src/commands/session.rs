use tauri::{AppHandle, State};

use crate::{
    models::{
        AppBootstrap, ConnectServerProfileRequest, ConnectServerProfileResult, ProfileIdRequest,
    },
    ActiveSessionState, CoverArtCacheState, PlaybackControllerState,
};

use super::common::service;

fn reset_playback_state(playback: &State<'_, PlaybackControllerState>, source: &str) {
    let mut controller = playback.0.lock().unwrap();
    controller.reset();
    log::info!("{source}: reset playback state to idle");
}

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_app_state(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<AppBootstrap, String> {
    let result = service(&app)?.bootstrap_app_state(&state.0).await?;
    if result.active_session.is_none() {
        reset_playback_state(&playback, "bootstrap_app_state");
    }
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn connect_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    playback: State<'_, PlaybackControllerState>,
    payload: ConnectServerProfileRequest,
) -> Result<ConnectServerProfileResult, String> {
    let result = service(&app)?
        .connect_server_profile(&state.0, payload)
        .await?;
    if matches!(result, ConnectServerProfileResult::Connected { .. }) {
        reset_playback_state(&playback, "connect_server_profile");
    }
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn delete_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: ProfileIdRequest,
) -> Result<AppBootstrap, String> {
    cover_art_cache.0.remove_profile(&payload.profile_id)?;
    let result = service(&app)?.delete_server_profile(&state.0, &payload.profile_id)?;
    if result.active_session.is_none() {
        reset_playback_state(&playback, "delete_server_profile");
    }
    Ok(result)
}
