use serde::{Deserialize, Serialize};
use tauri_specta::Event;

use super::{PlaybackStreamMode, SongResponse};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PlayingState {
    Idle,
    Playing,
    Paused,
    Stopped,
    Interrupted,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum InterruptReason {
    InitialLoad,
    StreamBufferingStall,
    Seeking,
    FullReload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GaplessState {
    Unavailable,
    Off,
    Idle,
    Preparing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GaplessStatus {
    pub state: GaplessState,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueSource {
    #[serde(rename_all = "camelCase")]
    Songs { songs: Vec<super::SongResponse> },
    #[serde(rename_all = "camelCase")]
    Album { album_id: String },
    #[serde(rename_all = "camelCase")]
    FolderAlbum {
        library_id: String,
        node_id: String,
        album_id: String,
    },
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSetQueueRequest {
    pub items: Vec<QueueSource>,
    pub current_index: Option<u32>,
    pub auto_play: bool,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSeekRequest {
    pub position_ms: u32,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSetPositionRequest {
    pub position_ms: u32,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSetVolumeRequest {
    pub volume: f32,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPlayQueueIndexRequest {
    pub index: u32,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackRemoveQueueIndexRequest {
    pub index: u32,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackMoveQueueIndexRequest {
    pub from_index: u32,
    pub to_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackError {
    pub message: String,
    pub handled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStreamRequestKind {
    Load,
    Prepare,
    ActivatePrepared,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStreamInfo {
    pub request_kind: PlaybackStreamRequestKind,
    pub song_id: String,
    pub song_path: Option<String>,
    pub source_content_type: Option<String>,
    pub source_suffix: Option<String>,
    pub source_bit_rate: Option<u32>,
    #[specta(type = Option<f64>)]
    pub source_size: Option<u64>,
    pub stream_mode: PlaybackStreamMode,
    pub raw_stream: bool,
    pub stream_format: Option<String>,
    pub stream_max_bit_rate: Option<u32>,
    pub stream_offset_seconds: Option<u32>,
    pub requested_position_ms: u32,
    pub local_start_position_ms: u32,
    pub autoplay: bool,
    pub stream_endpoint: String,
    pub stream_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackActualStreamInfo {
    pub codec: Option<String>,
    pub codec_profile: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bit_depth: Option<u32>,
    pub bitrate: Option<u32>,
    pub sample_format: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackOutputDevice {
    pub id: String,
    pub name: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStatus {
    pub playing_state: PlayingState,
    pub interrupt_reason: Option<InterruptReason>,
    pub pending_seek_position_ms: Option<u32>,
    pub gapless_status: GaplessStatus,
    pub queue: Vec<SongResponse>,
    pub current_index: Option<u32>,
    pub play_next_queue_len: u32,
    pub current_position_ms: u32,
    pub current_song_id: Option<String>,
    pub playback_error: Option<PlaybackError>,
    pub active_stream_info: Option<PlaybackStreamInfo>,
    pub prepared_stream_info: Option<PlaybackStreamInfo>,
    pub last_stream_request: Option<PlaybackStreamInfo>,
    pub actual_stream_info: Option<PlaybackActualStreamInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct MediaNotificationTap {}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackInsertAfterCurrentRequest {
    pub items: Vec<QueueSource>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackAppendToQueueRequest {
    pub items: Vec<QueueSource>,
}

impl PlaybackStatus {
    pub fn empty() -> Self {
        Self {
            playing_state: PlayingState::Idle,
            interrupt_reason: None,
            pending_seek_position_ms: None,
            gapless_status: GaplessStatus {
                state: GaplessState::Unavailable,
                message: "gapless: unavailable".to_string(),
            },
            queue: Vec::new(),
            current_index: None,
            play_next_queue_len: 0,
            current_position_ms: 0,
            current_song_id: None,
            playback_error: None,
            active_stream_info: None,
            prepared_stream_info: None,
            last_stream_request: None,
            actual_stream_info: None,
        }
    }
}
