use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::SongResponse;

const PLAYBACK_STATE_FILENAME: &str = "playback-state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStateFile {
    #[serde(default = "default_version")]
    pub version: u32,
    pub profile_id: String,
    pub queue: Vec<SongResponse>,
    pub current_index: Option<u32>,
    pub current_position_ms: u32,
}

pub trait PlaybackStatePersister: Send {
    fn save(&self, state: &PlaybackStateFile) -> Result<(), String>;
}

pub struct FilePlaybackStatePersister {
    path: PathBuf,
}

impl FilePlaybackStatePersister {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl PlaybackStatePersister for FilePlaybackStatePersister {
    fn save(&self, state: &PlaybackStateFile) -> Result<(), String> {
        save_playback_state_file(&self.path, state)
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct NoopPlaybackStatePersister;

impl PlaybackStatePersister for NoopPlaybackStatePersister {
    fn save(&self, _state: &PlaybackStateFile) -> Result<(), String> {
        Ok(())
    }
}

pub fn playback_state_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PLAYBACK_STATE_FILENAME)
}

pub fn load_playback_state_file(path: &Path) -> Result<Option<PlaybackStateFile>, String> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let file: PlaybackStateFile = serde_json::from_str(&contents)
                .map_err(|error| format!("Failed to parse playback state: {error}"))?;
            if file.version != 1 {
                log::warn!(
                    "load_playback_state_file: unsupported version {}, ignoring",
                    file.version
                );
                return Ok(None);
            }
            Ok(Some(file))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Failed to read playback state: {error}")),
    }
}

pub fn save_playback_state_file(path: &Path, state: &PlaybackStateFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create playback state directory: {error}"))?;
    }

    let json = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Failed to serialize playback state: {error}"))?;

    let mut file = fs::File::create(path)
        .map_err(|error| format!("Failed to open playback state file for writing: {error}"))?;
    file.write_all(json.as_bytes())
        .map_err(|error| format!("Failed to write playback state file: {error}"))?;
    Ok(())
}

fn default_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_song() -> SongResponse {
        SongResponse {
            id: "song-1".to_string(),
            parent_id: None,
            path: Some("/music/song1.mp3".to_string()),
            title: "Test Song".to_string(),
            album: Some("Test Album".to_string()),
            album_id: Some("album-1".to_string()),
            artist: Some("Test Artist".to_string()),
            artist_id: Some("artist-1".to_string()),
            cover_art_id: None,
            track: Some(1),
            disc_number: None,
            year: Some(2024),
            duration: Some(300),
            size: None,
            content_type: None,
            suffix: Some("mp3".to_string()),
            bit_rate: None,
            genre: None,
            created: None,
            starred: None,
            is_directory: false,
            media_type: Some("song".to_string()),
        }
    }

    #[test]
    fn round_trip_save_and_load() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("playback-state.json");

        let state = PlaybackStateFile {
            version: 1,
            profile_id: "profile-1".to_string(),
            queue: vec![sample_song()],
            current_index: Some(0),
            current_position_ms: 12345,
        };

        save_playback_state_file(&path, &state).unwrap();
        let loaded = load_playback_state_file(&path).unwrap().unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.profile_id, "profile-1");
        assert_eq!(loaded.queue.len(), 1);
        assert_eq!(loaded.queue[0].id, "song-1");
        assert_eq!(loaded.queue[0].title, "Test Song");
        assert_eq!(loaded.current_index, Some(0));
        assert_eq!(loaded.current_position_ms, 12345);
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");

        let result = load_playback_state_file(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_returns_none_for_unsupported_version() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("playback-state.json");

        let state = PlaybackStateFile {
            version: 99,
            profile_id: "profile-1".to_string(),
            queue: vec![],
            current_index: None,
            current_position_ms: 0,
        };

        save_playback_state_file(&path, &state).unwrap();
        let result = load_playback_state_file(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_returns_error_for_invalid_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("playback-state.json");

        fs::write(&path, "not valid json {{{").unwrap();
        let result = load_playback_state_file(&path);
        assert!(result.is_err());
    }
}
