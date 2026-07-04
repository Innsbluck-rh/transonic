use serde::{Deserialize, Serialize};
use tauri_specta::Event;
use typeshare::typeshare;

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

#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectPlaybackState {
    pub playing_state: PlayingState,
    // NOTE: queue / play_next_queue_len / current_position_ms are intentionally
    // required (no `#[serde(default)]`): they are always present on the wire, and
    // keeping them non-optional makes the generated Go non-pointer so the reducer's
    // index/length arithmetic stays free of nil checks. Only genuinely-optional
    // fields (current_index / current_song_id) default to None.
    pub queue: Vec<SongResponse>,
    #[serde(default)]
    pub current_index: Option<u32>,
    pub play_next_queue_len: u32,
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

fn is_false(value: &bool) -> bool {
    !*value
}

/// Outbound `playback.command.request` payload sent to the Connect server.
///
/// This is the single authoritative definition of the Connect queue command wire
/// contract; the Go server's `ConnectPlaybackCommand` is generated from it (see
/// `docs/known-issues.md` Tier 1). Only the fields relevant to a given `op` are
/// populated; the rest are omitted from the wire. `command_id` and `base_seq` are
/// filled in by `ConnectState::send_playback_command` just before sending.
#[typeshare]
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectPlaybackCommand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_seq: Option<u32>,
    pub op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<Vec<SongResponse>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<SongResponse>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_index: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_play: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_device_id: Option<String>,
}
