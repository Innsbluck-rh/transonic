use opensubsonic_client::api::search::{Search3Request as ApiSearch3Request, SearchApi};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error, normalize_optional_text, normalize_roles},
        json::{opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{AlbumListItem, ArtistSummary, SearchRequest, SearchResponse, SongResponse},
    ActiveSessionState,
};

use super::browse::RawSong;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSearchResult3 {
    #[serde(default, deserialize_with = "vec_or_single")]
    artist: Vec<RawSearchArtist>,
    #[serde(default, deserialize_with = "vec_or_single")]
    album: Vec<RawSearchAlbum>,
    #[serde(default, deserialize_with = "vec_or_single")]
    song: Vec<RawSong>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSearchArtist {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    album_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist_image_url: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    starred: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    music_brainz_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    sort_name: Option<String>,
    #[serde(default)]
    roles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSearchAlbum {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: String,
    artist: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
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

impl From<RawSearchArtist> for ArtistSummary {
    fn from(value: RawSearchArtist) -> Self {
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

impl From<RawSearchAlbum> for AlbumListItem {
    fn from(value: RawSearchAlbum) -> Self {
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

    parse_search_result3(response.payload.search_result3)
}

fn parse_search_result3(payload: Value) -> Result<SearchResponse, String> {
    let payload: RawSearchResult3 = value_as(payload)
        .map_err(|error| format!("Failed to parse the search payload: {error}"))?;

    Ok(SearchResponse {
        artists: payload
            .artist
            .into_iter()
            .map(ArtistSummary::from)
            .collect(),
        albums: payload.album.into_iter().map(AlbumListItem::from).collect(),
        songs: payload.song.into_iter().map(SongResponse::from).collect(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_search_result3;

    #[test]
    fn parse_search_result3_accepts_result_arrays() {
        let response = parse_search_result3(json!({
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
        }))
        .unwrap();

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
        let response = parse_search_result3(json!({
            "artist": {
                "id": 1001,
                "name": "Artist One"
            }
        }))
        .unwrap();

        assert_eq!(response.artists.len(), 1);
        assert_eq!(response.artists[0].id, "1001");
        assert!(response.albums.is_empty());
        assert!(response.songs.is_empty());
    }
}
