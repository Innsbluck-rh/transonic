use serde::{Deserialize, Serialize};
use tauri_specta::Event;

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
#[serde(rename_all = "camelCase")]
pub struct PlaybackQueueEntry {
    pub song_id: String,
    pub title: String,
    pub path: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<u32>,
    pub cover_art_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSetQueueRequest {
    pub entries: Vec<PlaybackQueueEntry>,
    pub current_index: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSeekRequest {
    pub position_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStatus {
    pub playing_state: PlayingState,
    pub interrupt_reason: Option<InterruptReason>,
    pub pending_seek_position_ms: Option<u32>,
    pub queue: Vec<PlaybackQueueEntry>,
    pub current_index: Option<u32>,
    pub current_position_ms: u32,
    pub current_song_id: Option<String>,
    pub error: Option<String>,
}

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
