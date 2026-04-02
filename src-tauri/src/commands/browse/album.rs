use opensubsonic_client::api::browsing::{BrowsingApi, GetAlbumRequest};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{opt_boolish, opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{AlbumSongsRequest, AlbumSongsResponse, SongResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
struct RawAlbum {
    #[serde(deserialize_with = "stringish")]
    id: String,
    #[serde(default, deserialize_with = "vec_or_single")]
    song: Vec<RawSong>,
}

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
pub async fn get_album_songs(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: AlbumSongsRequest,
) -> Result<AlbumSongsResponse, String> {
    let album_id = payload.id.trim();
    if album_id.is_empty() {
        return Err("id is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    load_album_songs_from_client(&client, album_id)
        .await
        .map_err(format_api_error)
}

pub(crate) async fn load_album_songs_from_client<C>(
    client: &C,
    album_id: &str,
) -> Result<AlbumSongsResponse, opensubsonic_client::ApiError>
where
    C: BrowsingApi + Send + Sync,
{
    let response = BrowsingApi::get_album(
        client,
        GetAlbumRequest {
            id: album_id.to_string(),
        },
    )
    .await?;

    parse_album(response.payload.album).map_err(opensubsonic_client::ApiError::Protocol)
}

pub(crate) fn parse_album(payload: Value) -> Result<AlbumSongsResponse, String> {
    let payload: RawAlbum =
        value_as(payload).map_err(|error| format!("Failed to parse the album payload: {error}"))?;

    Ok(AlbumSongsResponse {
        album_id: payload.id,
        songs: payload.song.into_iter().map(SongResponse::from).collect(),
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

    use super::parse_album;

    #[test]
    fn parse_album_accepts_array_songs() {
        let response = parse_album(json!({
            "id": "album-1",
            "song": [
                {
                    "id": "song-1",
                    "title": "Song One",
                    "track": "1",
                    "discNumber": "1",
                    "duration": "178",
                    "mediaType": " SONG "
                },
                {
                    "id": "song-2",
                    "title": "Song Two",
                    "track": 2
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.album_id, "album-1");
        assert_eq!(response.songs.len(), 2);
        assert_eq!(response.songs[0].id, "song-1");
        assert_eq!(response.songs[0].track, Some(1));
        assert_eq!(response.songs[0].disc_number, Some(1));
        assert_eq!(response.songs[0].duration, Some(178));
        assert_eq!(response.songs[0].media_type.as_deref(), Some("song"));
    }

    #[test]
    fn parse_album_accepts_single_song_objects() {
        let response = parse_album(json!({
            "id": 1001,
            "song": {
                "id": "song-1",
                "title": "Song One"
            }
        }))
        .unwrap();

        assert_eq!(response.album_id, "1001");
        assert_eq!(response.songs.len(), 1);
        assert_eq!(response.songs[0].title, "Song One");
    }

    #[test]
    fn parse_album_requires_song_title_without_fallback() {
        let error = parse_album(json!({
            "id": "album-1",
            "song": [
                {
                    "id": "song-1",
                    "name": "Song One"
                }
            ]
        }))
        .unwrap_err();

        assert!(error.contains("title"));
    }
}
