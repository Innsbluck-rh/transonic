use tauri::{AppHandle, State};

use crate::{
    commands::common::client,
    models::{CapabilityMatrix, PlaybackSeekRequest, PlaybackSetQueueRequest, PlaybackStatus},
    playback::PlaybackRuntimeContext,
    ActiveSessionState, PlaybackControllerState,
};

fn active_capability_matrix(state: &ActiveSessionState) -> Result<CapabilityMatrix, String> {
    let guard = state.0.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| "No active session is available.".to_string())?;
    Ok(session.capability_matrix.clone())
}

#[tauri::command]
#[specta::specta]
pub fn playback_get_state(
    state: State<'_, PlaybackControllerState>,
) -> Result<PlaybackStatus, String> {
    let mut controller = state.0.lock().unwrap();
    controller.synced_state()
}

#[tauri::command]
#[specta::specta]
pub fn playback_set_queue(
    state: State<'_, PlaybackControllerState>,
    payload: PlaybackSetQueueRequest,
) -> Result<(), String> {
    let mut controller = state.0.lock().unwrap();
    let result = controller.set_queue(payload).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_set_queue: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_play(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
    };

    let mut controller = playback.0.lock().unwrap();
    let result = controller.play(&runtime_context).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_play: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_pause(state: State<'_, PlaybackControllerState>) -> Result<(), String> {
    let mut controller = state.0.lock().unwrap();
    let result = controller.pause().map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_pause: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_stop(state: State<'_, PlaybackControllerState>) -> Result<(), String> {
    let mut controller = state.0.lock().unwrap();
    let result = controller.stop().map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_stop: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_seek(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    playback: State<'_, PlaybackControllerState>,
    payload: PlaybackSeekRequest,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
    };

    let mut controller = playback.0.lock().unwrap();
    let result = controller
        .seek(&runtime_context, payload.position_ms)
        .map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_seek: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_next(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
    };

    let mut controller = playback.0.lock().unwrap();
    let result = controller.next(&runtime_context).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_next: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_prev(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
    };

    let mut controller = playback.0.lock().unwrap();
    let result = controller.prev(&runtime_context).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_prev: failed: {error}");
    }
    result
}
