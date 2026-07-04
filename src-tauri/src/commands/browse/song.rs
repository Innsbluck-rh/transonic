use opensubsonic_client::api::browsing::{BrowsingApi, GetSongRequest};
use tauri::{AppHandle, State};

use crate::{
    commands::common::{client, format_api_error},
    models::{SongRequest, SongResponse},
    ActiveSessionState,
};

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

    Ok(SongResponse::from(response.payload.song))
}

#[cfg(test)]
mod tests {
    use opensubsonic_client::Child;
    use serde_json::json;

    fn parse_song(value: serde_json::Value) -> SongResponse {
        SongResponse::from(serde_json::from_value::<Child>(value).unwrap())
    }

    use crate::models::SongResponse;

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
        }));

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
        }));

        assert_eq!(response.id, "1001");
        assert_eq!(response.track, Some(4));
        assert_eq!(response.duration, Some(178));
        assert_eq!(response.disc_number, Some(1));
        assert_eq!(response.media_type.as_deref(), Some("song"));
    }
}
