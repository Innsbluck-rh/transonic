use tauri::{AppHandle, Manager};

use crate::{
    connect::ConnectState,
    models::{
        ConnectDevicePresence, ConnectRuntimeStatus, ConnectSharedPlaybackState,
        ConnectStateSnapshot, ConnectTransferPlaybackRequest,
    },
};

#[tauri::command]
#[specta::specta]
pub fn connect_get_state_snapshot(app: AppHandle) -> ConnectStateSnapshot {
    app.state::<ConnectState>().0.snapshot()
}

#[tauri::command]
#[specta::specta]
pub fn connect_get_runtime_status(app: AppHandle) -> ConnectRuntimeStatus {
    app.state::<ConnectState>().0.status()
}

#[tauri::command]
#[specta::specta]
pub fn connect_get_devices(app: AppHandle) -> Vec<ConnectDevicePresence> {
    app.state::<ConnectState>().0.devices()
}

#[tauri::command]
#[specta::specta]
pub fn connect_get_shared_playback(app: AppHandle) -> Option<ConnectSharedPlaybackState> {
    app.state::<ConnectState>().0.shared_playback()
}

#[tauri::command]
#[specta::specta]
pub fn connect_transfer_playback(
    app: AppHandle,
    payload: ConnectTransferPlaybackRequest,
) -> Result<(), String> {
    app.state::<ConnectState>()
        .0
        .request_transfer_playback(payload.target_device_id)
}
