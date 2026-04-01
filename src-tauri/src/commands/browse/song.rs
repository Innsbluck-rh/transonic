use opensubsonic_client::api::browsing::{BrowsingApi, GetSongRequest};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{opt_boolish, opt_stringish, opt_u32ish, stringish, value_as},
    },
    models::{SongRequest, SongResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSong {
    #[serde(deserialize_with = "stringish")]
    id: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    parent: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    path: Option<String>,
    title: String,
    album: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    album_id: Option<String>,
    artist: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist_id: Option<String>,
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    track: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    disc_number: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    year: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    duration: Option<u32>,
    #[serde(default, deserialize_with = "opt_boolish")]
    is_dir: Option<bool>,
    #[serde(default, deserialize_with = "opt_stringish")]
    media_type: Option<String>,
}

impl From<RawSong> for SongResponse {
    fn from(value: RawSong) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent,
            path: value.path,
            title: value.title,
            album: value.album,
            album_id: value.album_id,
            artist: value.artist,
            artist_id: value.artist_id,
            cover_art_id: value.cover_art,
            track: value.track,
            disc_number: value.disc_number,
            year: value.year,
            duration: value.duration,
            is_directory: value.is_dir.unwrap_or(false),
            media_type: normalize_media_type(value.media_type),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_song(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: SongRequest,
) -> Result<SongResponse, String> {
    let song_id = payload.id.trim();
    if song_id.is_empty() {
        return Err("id is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    let response = BrowsingApi::get_song(
        &client,
        GetSongRequest {
            id: song_id.to_string(),
        },
    )
    .await
    .map_err(format_api_error)?;

    parse_song(response.payload.song)
}

fn parse_song(payload: Value) -> Result<SongResponse, String> {
    let payload: RawSong =
        value_as(payload).map_err(|error| format!("Failed to parse the song payload: {error}"))?;

    Ok(SongResponse::from(payload))
}

fn normalize_media_type(media_type: Option<String>) -> Option<String> {
    media_type.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_song;

    #[test]
    fn parse_song_accepts_core_fields() {
        let response = parse_song(json!({
            "id": "song-1",
            "parent": "album-1",
            "title": "Song One",
            "album": "Album One",
            "artist": "Artist One",
            "coverArt": "cover-1",
            "track": 3,
            "year": 2024,
            "duration": 178,
            "mediaType": "song",
            "isDir": false
        }))
        .unwrap();

        assert_eq!(response.id, "song-1");
        assert_eq!(response.parent_id.as_deref(), Some("album-1"));
        assert_eq!(response.title, "Song One");
        assert_eq!(response.track, Some(3));
        assert_eq!(response.duration, Some(178));
        assert_eq!(response.media_type.as_deref(), Some("song"));
        assert!(!response.is_directory);
    }

    #[test]
    fn parse_song_accepts_string_numbers_and_normalizes_media_type() {
        let response = parse_song(json!({
            "id": 1001,
            "title": "Song One",
            "track": "04",
            "duration": "178",
            "discNumber": "1",
            "mediaType": " SONG "
        }))
        .unwrap();

        assert_eq!(response.id, "1001");
        assert_eq!(response.track, Some(4));
        assert_eq!(response.duration, Some(178));
        assert_eq!(response.disc_number, Some(1));
        assert_eq!(response.media_type.as_deref(), Some("song"));
    }

    #[test]
    fn parse_song_requires_title_without_fallback() {
        let error = parse_song(json!({
            "id": "song-1",
            "name": "Song One"
        }))
        .unwrap_err();

        assert!(error.contains("title"));
    }
}
