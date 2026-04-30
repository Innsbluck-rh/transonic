use tauri::{AppHandle, Manager};

use crate::{
    connect::ConnectState,
    models::{ConnectDeviceWithPlayback, ConnectRuntimeStatus},
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
