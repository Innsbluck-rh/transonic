use opensubsonic_client::api::browsing::{BrowsingApi, GetAlbumRequest};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{AlbumSongsRequest, AlbumSongsResponse, SongResponse},
    ActiveSessionState,
};

use super::RawSong;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbum {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: Option<String>,
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
    #[serde(default, deserialize_with = "vec_or_single")]
    song: Vec<RawSong>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_album_info(
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
        id: payload.id,
        name: payload.name,
        artist: payload.artist,
        artist_id: payload.artist_id,
        cover_art_id: payload.cover_art,
        song_count: payload.song_count,
        duration: payload.duration,
        play_count: payload.play_count,
        year: payload.year,
        genre: payload.genre,
        created: payload.created,
        starred: payload.starred,
        songs: payload.song.into_iter().map(SongResponse::from).collect(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_album;

    #[test]
    fn parse_album_preserves_album_metadata() {
        let response = parse_album(json!({
            "id": "album-1",
            "name": "Album One",
            "artist": "Artist One",
            "artistId": "artist-1",
            "coverArt": "cover-1",
            "songCount": "12",
            "duration": 3600,
            "playCount": "4",
            "year": "2024",
            "genre": "Rock",
            "created": "2024-01-01T00:00:00Z",
            "starred": "2024-06-01T00:00:00Z",
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

        assert_eq!(response.id, "album-1");
        assert_eq!(response.name.as_deref(), Some("Album One"));
        assert_eq!(response.artist.as_deref(), Some("Artist One"));
        assert_eq!(response.artist_id.as_deref(), Some("artist-1"));
        assert_eq!(response.cover_art_id.as_deref(), Some("cover-1"));
        assert_eq!(response.song_count, Some(12));
        assert_eq!(response.duration, Some(3600));
        assert_eq!(response.play_count, Some(4));
        assert_eq!(response.year, Some(2024));
        assert_eq!(response.genre.as_deref(), Some("Rock"));
        assert_eq!(response.created.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(response.starred.as_deref(), Some("2024-06-01T00:00:00Z"));
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

        assert_eq!(response.id, "1001");
        assert_eq!(response.name, None);
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
