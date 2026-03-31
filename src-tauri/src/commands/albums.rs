use opensubsonic_client::api::lists::ListsApi;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{value_as, vec_or_single},
    },
    models::{AlbumListItem, AlbumListRequest, AlbumListResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
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
    cover_art: Option<String>,
    year: Option<u32>,
}

impl From<RawAlbum> for AlbumListItem {
    fn from(value: RawAlbum) -> Self {
        Self {
            id: value.id,
            name: value.name,
            artist: value.artist,
            cover_art_id: value.cover_art,
            year: value.year,
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
                    "coverArt": "cover-1",
                    "year": 2024
                }
            ]
        }))
        .unwrap();

        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].id, "album-1");
        assert_eq!(albums[0].name, "First Album");
        assert_eq!(albums[0].artist.as_deref(), Some("First Artist"));
        assert_eq!(albums[0].cover_art_id.as_deref(), Some("cover-1"));
        assert_eq!(albums[0].year, Some(2024));
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
    }
}
