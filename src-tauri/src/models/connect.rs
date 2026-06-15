use serde::{Deserialize, Serialize};
use tauri_specta::Event;

use super::{PlaybackStatus, PlayingState, SongResponse};

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

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectStateSnapshot {
    pub runtime: ConnectRuntimeStatus,
    pub devices: Vec<ConnectDevicePresence>,
    pub shared_playback: Option<ConnectSharedPlaybackState>,
}

#[derive(Debug, Clone, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ConnectStateUpdated {
    pub runtime: ConnectRuntimeStatus,
    pub devices: Vec<ConnectDevicePresence>,
    pub shared_playback: Option<ConnectSharedPlaybackState>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectPlaybackState {
    pub playing_state: PlayingState,
    #[serde(default)]
    pub queue: Vec<SongResponse>,
    #[serde(default)]
    pub current_index: Option<u32>,
    #[serde(default)]
    pub play_next_queue_len: u32,
    #[serde(default)]
    pub current_position_ms: u32,
    #[serde(default)]
    pub current_song_id: Option<String>,
}

impl From<PlaybackStatus> for ConnectPlaybackState {
    fn from(status: PlaybackStatus) -> Self {
        Self {
            playing_state: status.playing_state,
            queue: status.queue,
            current_index: status.current_index,
            play_next_queue_len: status.play_next_queue_len,
            current_position_ms: status.current_position_ms,
            current_song_id: status.current_song_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectSharedPlaybackState {
    pub seq: u32,
    pub active_device_id: Option<String>,
    pub state: ConnectPlaybackState,
    pub updated_at: String,
    pub updated_by_device_id: Option<String>,
    pub update_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectTransferPlaybackRequest {
    pub target_device_id: String,
}
