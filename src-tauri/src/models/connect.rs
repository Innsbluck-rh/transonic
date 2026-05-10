use serde::{Deserialize, Serialize};
use tauri_specta::Event;

use super::PlaybackStatus;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRuntimeStatus {
    pub enabled: bool,
    pub connected: bool,
    pub message: Option<String>,
    pub server_url: Option<String>,
    pub device_id: Option<String>,
    pub seq: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectDevicePresence {
    pub device_id: String,
    pub display_name: String,
    pub platform: String,
    pub app_version: String,
    pub last_seen_at: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectPlaybackDeviceState {
    pub seq: u32,
    pub source_device_id: String,
    pub state: PlaybackStatus,
    pub position_ms: u32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectDeviceWithPlayback {
    pub device: ConnectDevicePresence,
    pub playback: Option<ConnectPlaybackDeviceState>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectPlaybackTakeoverRequest {
    pub source_device_id: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRemotePlaybackRequest {
    pub target_device_id: String,
}

#[derive(Debug, Clone, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ConnectDevicesUpdated {
    pub devices: Vec<ConnectDeviceWithPlayback>,
}
