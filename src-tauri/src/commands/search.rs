use opensubsonic_client::api::search::{
    Search3Request as ApiSearch3Request, SearchAlbum, SearchApi, SearchArtist, SearchResult3,
};
use tauri::{AppHandle, State};

use crate::{
    commands::common::{client, format_api_error, normalize_optional_text, normalize_roles},
    models::{AlbumListItem, ArtistSummary, SearchRequest, SearchResponse, SongResponse},
    ActiveSessionState,
};

impl From<SearchArtist> for ArtistSummary {
    fn from(value: SearchArtist) -> Self {
        Self {
            id: value.id,
            name: value.name,
            cover_art_id: normalize_optional_text(value.cover_art),
            album_count: value.album_count,
            artist_image_url: normalize_optional_text(value.artist_image_url),
            starred: normalize_optional_text(value.starred),
            music_brainz_id: normalize_optional_text(value.music_brainz_id),
            sort_name: normalize_optional_text(value.sort_name),
            roles: normalize_roles(value.roles),
        }
    }
}

impl From<SearchAlbum> for AlbumListItem {
    fn from(value: SearchAlbum) -> Self {
        Self {
            id: value.id,
            name: value.name,
            artist: normalize_optional_text(value.artist),
            artist_id: normalize_optional_text(value.artist_id),
            cover_art_id: normalize_optional_text(value.cover_art),
            song_count: value.song_count,
            duration: value.duration,
            play_count: value.play_count,
            year: value.year,
            genre: normalize_optional_text(value.genre),
            created: normalize_optional_text(value.created),
            starred: normalize_optional_text(value.starred),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn search(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: SearchRequest,
) -> Result<SearchResponse, String> {
    let client = client(&app, &state.0)?;
    let response = SearchApi::search3(
        &client,
        ApiSearch3Request {
            query: payload.query,
            artist_count: None,
            artist_offset: None,
            album_count: None,
            album_offset: None,
            song_count: None,
            song_offset: None,
            music_folder_id: None,
        },
    )
    .await
    .map_err(format_api_error)?;

    Ok(search_result3_response(response.payload.search_result3))
}

fn search_result3_response(payload: SearchResult3) -> SearchResponse {
    SearchResponse {
        artists: payload
            .artist
            .into_iter()
            .map(ArtistSummary::from)
            .collect(),
        albums: payload.album.into_iter().map(AlbumListItem::from).collect(),
        songs: payload.song.into_iter().map(SongResponse::from).collect(),
    }
}

#[cfg(test)]
mod tests {
    use opensubsonic_client::api::search::SearchResult3;
    use serde_json::json;

    use super::search_result3_response;

    fn parse_payload(value: serde_json::Value) -> SearchResult3 {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn parse_search_result3_accepts_result_arrays() {
        let response = search_result3_response(parse_payload(json!({
            "artist": [
                {
                    "id": "artist-1",
                    "name": "Artist One",
                    "coverArt": "artist-cover",
                    "albumCount": "2"
                }
            ],
            "album": [
                {
                    "id": "album-1",
                    "name": "Album One",
                    "artist": "Artist One",
                    "artistId": "artist-1",
                    "coverArt": "album-cover",
                    "songCount": "10"
                }
            ],
            "song": [
                {
                    "id": "song-1",
                    "title": "Song One",
                    "album": "Album One",
                    "albumId": "album-1",
                    "artist": "Artist One",
                    "artistId": "artist-1",
                    "coverArt": "song-cover",
                    "track": "1",
                    "duration": "180"
                }
            ]
        })));

        assert_eq!(response.artists.len(), 1);
        assert_eq!(response.artists[0].name, "Artist One");
        assert_eq!(response.artists[0].album_count, Some(2));
        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].name, "Album One");
        assert_eq!(response.albums[0].song_count, Some(10));
        assert_eq!(response.songs.len(), 1);
        assert_eq!(response.songs[0].title, "Song One");
        assert_eq!(response.songs[0].track, Some(1));
        assert_eq!(response.songs[0].duration, Some(180));
    }

    #[test]
    fn parse_search_result3_accepts_single_objects_and_missing_sections() {
        let response = search_result3_response(parse_payload(json!({
            "artist": {
                "id": 1001,
                "name": "Artist One"
            }
        })));

        assert_eq!(response.artists.len(), 1);
        assert_eq!(response.artists[0].id, "1001");
        assert!(response.albums.is_empty());
        assert!(response.songs.is_empty());
    }
}
