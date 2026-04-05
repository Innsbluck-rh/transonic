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

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSetQueueRequest {
    pub entries: Vec<SongResponse>,
    pub current_index: Option<u32>,
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

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPlayAlbumRequest {
    pub album_id: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPlayFolderAlbumRequest {
    pub library_id: String,
    pub node_id: String,
    pub album_id: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPlaySongsRequest {
    pub songs: Vec<super::SongResponse>,
    pub start_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStatus {
    pub playing_state: PlayingState,
    pub interrupt_reason: Option<InterruptReason>,
    pub pending_seek_position_ms: Option<u32>,
    pub queue: Vec<SongResponse>,
    pub current_index: Option<u32>,
    pub current_position_ms: u32,
    pub current_song_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct MediaNotificationTap {}

impl PlaybackStatus {
    pub fn empty() -> Self {
        Self {
            playing_state: PlayingState::Idle,
            interrupt_reason: None,
            pending_seek_position_ms: None,
            queue: Vec::new(),
            current_index: None,
            current_position_ms: 0,
            current_song_id: None,
            error: None,
        }
    }
}
