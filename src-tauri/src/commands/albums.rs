use opensubsonic_client::api::lists::ListsApi;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{opt_stringish, opt_u32ish, value_as, vec_or_single},
    },
    models::{AlbumListItem, AlbumListRequest, AlbumListResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbums {
    #[serde(default, deserialize_with = "vec_or_single")]
    album: Vec<RawAlbum>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbum {
    id: String,
    name: String,
    artist: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist_id: Option<String>,
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    song_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    duration: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    play_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    year: Option<u32>,
    genre: Option<String>,
    created: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    starred: Option<String>,
}

impl From<RawAlbum> for AlbumListItem {
    fn from(value: RawAlbum) -> Self {
        Self {
            id: value.id,
            name: value.name,
            artist: value.artist,
            artist_id: value.artist_id,
            cover_art_id: value.cover_art,
            song_count: value.song_count,
            duration: value.duration,
            play_count: value.play_count,
            year: value.year,
            genre: value.genre,
            created: value.created,
            starred: value.starred,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_album_list(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: AlbumListRequest,
) -> Result<AlbumListResponse, String> {
    let context = payload.context.clone();
    let client = client(&app, &state.0)?;
    let response = client
        .get_album_list2(payload.to_open_subsonic_request()?)
        .await
        .map_err(format_api_error)?;

    Ok(AlbumListResponse {
        context,
        albums: parse_albums(response.payload.album_list2)?,
    })
}

fn parse_albums(payload: Value) -> Result<Vec<AlbumListItem>, String> {
    let payload: RawAlbums = value_as(payload)
        .map_err(|error| format!("Failed to parse the album list payload: {error}"))?;

    Ok(payload.album.into_iter().map(AlbumListItem::from).collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_albums;

    #[test]
    fn parse_albums_accepts_array_payloads() {
        let albums = parse_albums(json!({
            "album": [
                {
                    "id": "album-1",
                    "name": "First Album",
                    "artist": "First Artist",
                    "artistId": "artist-1",
                    "coverArt": "cover-1",
                    "songCount": 10,
                    "duration": 2400,
                    "playCount": "5",
                    "year": 2024,
                    "genre": "Rock",
                    "created": "2024-01-01T00:00:00Z",
                    "starred": "2024-06-01T00:00:00Z"
                }
            ]
        }))
        .unwrap();

        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].id, "album-1");
        assert_eq!(albums[0].name, "First Album");
        assert_eq!(albums[0].artist.as_deref(), Some("First Artist"));
        assert_eq!(albums[0].artist_id.as_deref(), Some("artist-1"));
        assert_eq!(albums[0].cover_art_id.as_deref(), Some("cover-1"));
        assert_eq!(albums[0].song_count, Some(10));
        assert_eq!(albums[0].duration, Some(2400));
        assert_eq!(albums[0].play_count, Some(5));
        assert_eq!(albums[0].year, Some(2024));
        assert_eq!(albums[0].genre.as_deref(), Some("Rock"));
        assert_eq!(albums[0].created.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(albums[0].starred.as_deref(), Some("2024-06-01T00:00:00Z"));
    }

    #[test]
    fn parse_albums_accepts_single_album_objects() {
        let albums = parse_albums(json!({
            "album": {
                "id": "album-1",
                "name": "First Album"
            }
        }))
        .unwrap();

        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].name, "First Album");
        assert_eq!(albums[0].artist, None);
        assert_eq!(albums[0].song_count, None);
    }
}
