use opensubsonic_client::api::browsing::{BrowsingApi, GetMusicDirectoryRequest};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{opt_boolish, opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{MusicDirectoryChild, MusicDirectoryRequest, MusicDirectoryResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
struct RawDirectory {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: String,
    #[serde(default, deserialize_with = "vec_or_single")]
    child: Vec<RawChild>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChild {
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
    #[serde(default, deserialize_with = "opt_boolish")]
    is_dir: Option<bool>,
    #[serde(default, deserialize_with = "opt_stringish")]
    media_type: Option<String>,
}

impl From<RawChild> for MusicDirectoryChild {
    fn from(value: RawChild) -> Self {
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
            is_directory: value.is_dir.unwrap_or(false),
            media_type: normalize_media_type(value.media_type),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_music_directory(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: MusicDirectoryRequest,
) -> Result<MusicDirectoryResponse, String> {
    let directory_id = payload.id.trim();
    if directory_id.is_empty() {
        return Err("id is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    let response = BrowsingApi::get_music_directory(
        &client,
        GetMusicDirectoryRequest {
            id: directory_id.to_string(),
        },
    )
    .await
    .map_err(format_api_error)?;

    parse_directory(response.payload.directory)
}

fn parse_directory(payload: Value) -> Result<MusicDirectoryResponse, String> {
    let payload: RawDirectory = value_as(payload)
        .map_err(|error| format!("Failed to parse the music directory payload: {error}"))?;

    Ok(MusicDirectoryResponse {
        id: payload.id,
        name: payload.name,
        children: payload
            .child
            .into_iter()
            .map(MusicDirectoryChild::from)
            .collect(),
    })
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

    use super::parse_directory;

    #[test]
    fn parse_directory_accepts_array_payloads() {
        let response = parse_directory(json!({
            "id": "artist-1",
            "name": "Artist One",
            "child": [
                {
                    "id": "album-1",
                    "isDir": true,
                    "title": "Album One",
                    "artist": "Artist One",
                    "coverArt": "cover-1",
                    "year": "2024"
                },
                {
                    "id": "song-1",
                    "isDir": false,
                    "title": "Song One",
                    "artist": "Artist One"
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.id, "artist-1");
        assert_eq!(response.name, "Artist One");
        assert_eq!(response.children.len(), 2);
        assert_eq!(response.children[0].title, "Album One");
        assert_eq!(
            response.children[0].cover_art_id.as_deref(),
            Some("cover-1")
        );
        assert_eq!(response.children[0].year, Some(2024));
        assert!(response.children[0].is_directory);
        assert_eq!(response.children[1].title, "Song One");
        assert_eq!(response.children[1].media_type, None);
        assert!(!response.children[1].is_directory);
    }

    #[test]
    fn parse_directory_accepts_single_child_objects() {
        let response = parse_directory(json!({
            "id": "artist-1",
            "name": "Artist One",
            "child": {
                "id": "song-1",
                "title": "Song One",
                "mediaType": " SONG ",
                "isDir": false
            }
        }))
        .unwrap();

        assert_eq!(response.children.len(), 1);
        assert_eq!(response.children[0].media_type.as_deref(), Some("song"));
    }

    #[test]
    fn parse_directory_requires_title_without_fallback() {
        let error = parse_directory(json!({
            "id": "artist-1",
            "name": "Artist One",
            "child": {
                "id": "song-1",
                "name": "Song One"
            }
        }))
        .unwrap_err();

        assert!(error.contains("title"));
    }
}
