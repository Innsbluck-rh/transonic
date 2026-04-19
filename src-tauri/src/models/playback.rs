use serde::{Deserialize, Serialize};
use tauri_specta::Event;

use super::SongResponse;

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
pub struct PlaybackPlayQueueIndexRequest {
    pub index: u32,
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
    pub current_position_ms: u32,
    pub current_song_id: Option<String>,
    pub error: Option<String>,
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
            current_position_ms: 0,
            current_song_id: None,
            error: None,
        }
    }
}
