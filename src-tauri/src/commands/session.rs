use tauri::{AppHandle, Manager, State};

use crate::{
    models::{
        AppBootstrap, ConnectServerProfileRequest, ConnectServerProfileResult, ProfileIdRequest,
    },
    playback_state::{load_playback_state_file, playback_state_path},
    ActiveSessionState, AppSettingsState, ArtistImageCacheState, CoverArtCacheState,
    PlaybackControllerState,
};

use super::common::service;

fn apply_settings_snapshot(
    settings: &State<'_, AppSettingsState>,
    bootstrap: &mut AppBootstrap,
) -> Result<(), String> {
    let guard = settings
        .0
        .lock()
        .map_err(|_| "The app settings state is unavailable.".to_string())?;
    let (app_settings, settings_origin) = guard.snapshot();
    bootstrap.settings = app_settings;
    bootstrap.settings_origin = settings_origin;
    Ok(())
}

fn apply_playback_capabilities(
    playback: &State<'_, PlaybackControllerState>,
    bootstrap: &mut AppBootstrap,
) -> Result<(), String> {
    let controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    bootstrap.playback_capabilities = controller.capabilities();
    Ok(())
}

fn reset_playback_state(playback: &State<'_, PlaybackControllerState>, source: &str) {
    let mut controller = playback.0.lock().unwrap();
    controller.reset();
    log::info!("{source}: reset playback state to idle");
}

fn restore_playback_state(
    app: &AppHandle,
    playback: &State<'_, PlaybackControllerState>,
    active_profile_id: &str,
) {
    if is_playback_state_initialized_for_profile(playback, active_profile_id) {
        log::info!(
            "restore_playback_state: keeping live playback state for profile {active_profile_id}"
        );
        return;
    }

    let config_dir = match app.path().app_config_dir() {
        Ok(dir) => dir,
        Err(error) => {
            log::warn!("restore_playback_state: failed to resolve config dir: {error}");
            return;
        }
    };

    let path = playback_state_path(&config_dir);
    let state_file = match load_playback_state_file(&path) {
        Ok(Some(file)) => file,
        Ok(None) => {
            log::info!("restore_playback_state: no persisted playback state found");
            set_active_profile_id(playback, active_profile_id);
            return;
        }
        Err(error) => {
            log::warn!("restore_playback_state: failed to load playback state: {error}");
            set_active_profile_id(playback, active_profile_id);
            return;
        }
    };

    if state_file.profile_id != active_profile_id {
        log::info!(
            "restore_playback_state: profile mismatch (saved={}, active={}), discarding",
            state_file.profile_id,
            active_profile_id,
        );
        set_active_profile_id(playback, active_profile_id);
        return;
    }

    let mut controller = playback.0.lock().unwrap();
    controller.restore_state(
        state_file.profile_id,
        state_file.queue,
        state_file.current_index,
        state_file.current_position_ms,
    );
    log::info!("restore_playback_state: restored playback state for profile {active_profile_id}");
}

fn is_playback_state_initialized_for_profile(
    playback: &State<'_, PlaybackControllerState>,
    profile_id: &str,
) -> bool {
    let controller = playback.0.lock().unwrap();
    controller.is_initialized_for_profile(profile_id)
}

fn set_active_profile_id(playback: &State<'_, PlaybackControllerState>, profile_id: &str) {
    let mut controller = playback.0.lock().unwrap();
    controller.set_active_profile_id(Some(profile_id.to_string()));
}

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_app_state(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    settings: State<'_, AppSettingsState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<AppBootstrap, String> {
    if let Err(error) = super::backup::maybe_auto_import_server_backup(&app, &settings) {
        log::warn!("bootstrap_app_state: failed to auto import server backup: {error}");
    }

    let mut result = service(&app)?.bootstrap_app_state(&state.0).await?;
    apply_playback_capabilities(&playback, &mut result)?;
    apply_settings_snapshot(&settings, &mut result)?;
    if let Some(ref active_session) = result.active_session {
        restore_playback_state(&app, &playback, &active_session.profile_id);
    } else {
        reset_playback_state(&playback, "bootstrap_app_state");
    }
    crate::connect::restart(&app);
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
    if let ConnectServerProfileResult::Connected {
        ref active_session, ..
    } = result
    {
        reset_playback_state(&playback, "connect_server_profile");
        set_active_profile_id(&playback, &active_session.profile_id);
        crate::connect::restart(&app);
    }
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn delete_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    settings: State<'_, AppSettingsState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    artist_image_cache: State<'_, ArtistImageCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: ProfileIdRequest,
) -> Result<AppBootstrap, String> {
    cover_art_cache.0.remove_profile(&payload.profile_id)?;
    artist_image_cache.0.remove_profile(&payload.profile_id)?;
    let mut result = service(&app)?.delete_server_profile(&state.0, &payload.profile_id)?;
    apply_playback_capabilities(&playback, &mut result)?;
    apply_settings_snapshot(&settings, &mut result)?;
    if result.active_session.is_none() {
        reset_playback_state(&playback, "delete_server_profile");
    }
    crate::connect::restart(&app);
    Ok(result)
}
