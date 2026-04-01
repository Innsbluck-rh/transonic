use serde::{Deserialize, Serialize};
use tauri_specta::Event;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    Idle,
    Loading,
    Playing,
    Paused,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackQueueEntry {
    pub song_id: String,
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
    pub state: PlaybackState,
    pub queue: Vec<PlaybackQueueEntry>,
    pub current_index: Option<u32>,
    pub current_position_ms: u32,
    pub current_song_id: Option<String>,
    pub error: Option<String>,
}

impl PlaybackStatus {
    pub fn empty() -> Self {
        Self {
            state: PlaybackState::Idle,
            queue: Vec::new(),
            current_index: None,
            current_position_ms: 0,
            current_song_id: None,
            error: None,
        }
    }
}
