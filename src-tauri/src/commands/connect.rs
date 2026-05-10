use tauri::{AppHandle, Manager};

use crate::{
    connect::ConnectState,
    models::{
        ConnectDeviceWithPlayback, ConnectPlaybackTakeoverRequest, ConnectRemotePlaybackRequest,
        ConnectRuntimeStatus,
    },
};

#[tauri::command]
#[specta::specta]
pub fn connect_get_runtime_status(app: AppHandle) -> ConnectRuntimeStatus {
    app.state::<ConnectState>().0.status()
}

#[tauri::command]
#[specta::specta]
pub fn connect_get_devices_with_playback(app: AppHandle) -> Vec<ConnectDeviceWithPlayback> {
    app.state::<ConnectState>().0.devices_with_playback()
}

#[tauri::command]
#[specta::specta]
pub fn connect_takeover_playback(
    app: AppHandle,
    payload: ConnectPlaybackTakeoverRequest,
) -> Result<(), String> {
    app.state::<ConnectState>()
        .0
        .request_handoff(payload.source_device_id)
}

#[tauri::command]
#[specta::specta]
pub fn connect_pause_device_playback(
    app: AppHandle,
    payload: ConnectRemotePlaybackRequest,
) -> Result<(), String> {
    app.state::<ConnectState>()
        .0
        .pause_remote_device(payload.target_device_id)
}
