use opensubsonic_client::api::browsing::{
    BrowsingApi, GetArtistInfo2Request, GetArtistRequest, GetArtistsRequest,
    GetMusicDirectoryRequest,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error, normalize_media_type, normalize_optional_text, normalize_roles, trim_text},
        json::{opt_boolish, opt_stringish, opt_u32ish, opt_u64ish, stringish, value_as, vec_or_single},
    },
    models::{
        ArtistAlbum, ArtistGroup, ArtistInfo2Request, ArtistInfo2Response, ArtistRequest,
        ArtistResponse, ArtistSummary, ArtistsRequest, ArtistsResponse, MusicDirectoryChild,
        MusicDirectoryRequest, MusicDirectoryResponse,
    },
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
    #[serde(default, deserialize_with = "opt_u64ish")]
    size: Option<u64>,
    content_type: Option<String>,
    suffix: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    bit_rate: Option<u32>,
    genre: Option<String>,
    created: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    starred: Option<String>,
    #[serde(default, deserialize_with = "opt_boolish")]
    is_dir: Option<bool>,
    #[serde(default, deserialize_with = "opt_stringish")]
    media_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawArtists {
    #[serde(default, deserialize_with = "opt_stringish")]
    ignored_articles: Option<String>,
    #[serde(default, deserialize_with = "vec_or_single")]
    index: Vec<RawArtistIndex>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawArtistIndex {
    name: String,
    #[serde(default, deserialize_with = "vec_or_single")]
    artist: Vec<RawArtistSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawArtistSummary {
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
struct RawArtist {
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
    #[serde(default, deserialize_with = "vec_or_single")]
    album: Vec<RawArtistAlbum>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawArtistAlbum {
    #[serde(deserialize_with = "stringish")]
    id: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    parent: Option<String>,
    title: Option<String>,
    album: Option<String>,
    name: Option<String>,
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
    #[serde(default, deserialize_with = "opt_stringish")]
    starred: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawArtistInfo2 {
    #[serde(default, deserialize_with = "opt_stringish")]
    biography: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    music_brainz_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    last_fm_url: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    small_image_url: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    medium_image_url: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    large_image_url: Option<String>,
    #[serde(default, deserialize_with = "vec_or_single")]
    similar_artist: Vec<RawArtistSummary>,
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
            size: value.size,
            content_type: value.content_type,
            suffix: value.suffix,
            bit_rate: value.bit_rate,
            genre: value.genre,
            created: value.created,
            starred: value.starred,
            is_directory: value.is_dir.unwrap_or(false),
            media_type: normalize_media_type(value.media_type),
        }
    }
}

impl From<RawArtistSummary> for ArtistSummary {
    fn from(value: RawArtistSummary) -> Self {
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

impl From<RawArtistIndex> for ArtistGroup {
    fn from(value: RawArtistIndex) -> Self {
        Self {
            name: value.name,
            artists: value.artist.into_iter().map(ArtistSummary::from).collect(),
        }
    }
}

impl From<RawArtistAlbum> for ArtistAlbum {
    fn from(value: RawArtistAlbum) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent,
            title: normalize_optional_text(value.title),
            album: normalize_optional_text(value.album),
            name: normalize_optional_text(value.name),
            artist: normalize_optional_text(value.artist),
            artist_id: normalize_optional_text(value.artist_id),
            cover_art_id: normalize_optional_text(value.cover_art),
            song_count: value.song_count,
            duration: value.duration,
            play_count: value.play_count,
            year: value.year,
            genre: normalize_optional_text(value.genre),
            starred: normalize_optional_text(value.starred),
        }
    }
}

impl From<RawArtist> for ArtistResponse {
    fn from(value: RawArtist) -> Self {
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
            albums: value.album.into_iter().map(ArtistAlbum::from).collect(),
        }
    }
}

impl From<RawArtistInfo2> for ArtistInfo2Response {
    fn from(value: RawArtistInfo2) -> Self {
        Self {
            biography: normalize_optional_text(value.biography),
            music_brainz_id: normalize_optional_text(value.music_brainz_id),
            last_fm_url: normalize_optional_text(value.last_fm_url),
            small_image_url: normalize_optional_text(value.small_image_url),
            medium_image_url: normalize_optional_text(value.medium_image_url),
            large_image_url: normalize_optional_text(value.large_image_url),
            similar_artists: value
                .similar_artist
                .into_iter()
                .map(ArtistSummary::from)
                .collect(),
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

#[tauri::command]
#[specta::specta]
pub async fn get_artists(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: Option<ArtistsRequest>,
) -> Result<ArtistsResponse, String> {
    let music_folder_id = payload
        .and_then(|payload| payload.music_folder_id)
        .and_then(|value| trim_text(&value));

    let client = client(&app, &state.0)?;
    let response = BrowsingApi::get_artists(&client, GetArtistsRequest { music_folder_id })
        .await
        .map_err(format_api_error)?;

    parse_artists(response.payload.artists)
}

#[tauri::command]
#[specta::specta]
pub async fn get_artist(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: ArtistRequest,
) -> Result<ArtistResponse, String> {
    let artist_id = payload.id.trim();
    if artist_id.is_empty() {
        return Err("id is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    let response = BrowsingApi::get_artist(
        &client,
        GetArtistRequest {
            id: artist_id.to_string(),
        },
    )
    .await
    .map_err(format_api_error)?;

    parse_artist(response.payload.artist)
}

#[tauri::command]
#[specta::specta]
pub async fn get_artist_info2(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: ArtistInfo2Request,
) -> Result<ArtistInfo2Response, String> {
    let artist_id = payload.id.trim();
    if artist_id.is_empty() {
        return Err("id is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    let response = BrowsingApi::get_artist_info2(
        &client,
        GetArtistInfo2Request {
            id: artist_id.to_string(),
            count: payload.count,
            include_not_present: payload.include_not_present,
        },
    )
    .await
    .map_err(format_api_error)?;

    parse_artist_info2(response.payload.artist_info2)
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

fn parse_artists(payload: Value) -> Result<ArtistsResponse, String> {
    let payload: RawArtists = value_as(payload)
        .map_err(|error| format!("Failed to parse the artists payload: {error}"))?;

    Ok(ArtistsResponse {
        ignored_articles: normalize_optional_text(payload.ignored_articles),
        indexes: payload.index.into_iter().map(ArtistGroup::from).collect(),
    })
}

fn parse_artist(payload: Value) -> Result<ArtistResponse, String> {
    let payload: RawArtist = value_as(payload)
        .map_err(|error| format!("Failed to parse the artist payload: {error}"))?;

    Ok(ArtistResponse::from(payload))
}

fn parse_artist_info2(payload: Value) -> Result<ArtistInfo2Response, String> {
    let payload: RawArtistInfo2 = value_as(payload)
        .map_err(|error| format!("Failed to parse the artist info payload: {error}"))?;

    Ok(ArtistInfo2Response::from(payload))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{parse_artist, parse_artist_info2, parse_artists, parse_directory};

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

    #[test]
    fn parse_artists_accepts_single_index_objects() {
        let response = parse_artists(json!({
            "ignoredArticles": "The An A",
            "index": {
                "name": "A",
                "artist": {
                    "id": "artist-1",
                    "name": "Artist One",
                    "coverArt": "ar-1",
                    "albumCount": "2",
                    "roles": [" artist ", ""]
                }
            }
        }))
        .unwrap();

        assert_eq!(response.ignored_articles.as_deref(), Some("The An A"));
        assert_eq!(response.indexes.len(), 1);
        assert_eq!(response.indexes[0].name, "A");
        assert_eq!(response.indexes[0].artists[0].id, "artist-1");
        assert_eq!(response.indexes[0].artists[0].album_count, Some(2));
        assert_eq!(response.indexes[0].artists[0].roles, vec!["artist"]);
    }

    #[test]
    fn parse_artist_accepts_album_arrays() {
        let response = parse_artist(json!({
            "id": "artist-1",
            "name": "Artist One",
            "coverArt": "ar-1",
            "albumCount": "2",
            "musicBrainzId": "mbid-1",
            "roles": ["artist"],
            "album": [
                {
                    "id": "album-1",
                    "parent": "root-1",
                    "album": "Album One",
                    "title": "Album One Title",
                    "name": "Album One Name",
                    "artist": "Artist One",
                    "artistId": "artist-1",
                    "coverArt": "al-1",
                    "songCount": "12",
                    "duration": 3600,
                    "playCount": "4",
                    "year": "2024",
                    "genre": "Rock",
                    "starred": "2024-01-01T00:00:00Z"
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.id, "artist-1");
        assert_eq!(response.album_count, Some(2));
        assert_eq!(response.music_brainz_id.as_deref(), Some("mbid-1"));
        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].title.as_deref(), Some("Album One Title"));
        assert_eq!(response.albums[0].song_count, Some(12));
    }

    #[test]
    fn parse_artist_info2_accepts_similar_artist_objects() {
        let response = parse_artist_info2(json!({
            "biography": " Artist bio ",
            "musicBrainzId": "mbid-1",
            "largeImageUrl": "https://example.com/artist.jpg",
            "similarArtist": {
                "id": "artist-2",
                "name": "Artist Two",
                "albumCount": "3"
            }
        }))
        .unwrap();

        assert_eq!(response.biography.as_deref(), Some("Artist bio"));
        assert_eq!(
            response.large_image_url.as_deref(),
            Some("https://example.com/artist.jpg")
        );
        assert_eq!(response.similar_artists.len(), 1);
        assert_eq!(response.similar_artists[0].id, "artist-2");
        assert_eq!(response.similar_artists[0].album_count, Some(3));
    }
}
