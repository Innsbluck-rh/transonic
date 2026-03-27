use tauri::{AppHandle, Manager, Runtime, State};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use opensubsonic_client::{
    api::{
        browsing::{
            BrowsingApi, GetIndexesRequest, GetMusicDirectoryRequest, GetMusicFoldersRequest,
        },
        lists::ListsApi,
        retrieval::{GetCoverArtRequest, RetrievalApi},
    },
    ApiError, PreparedBinaryRequest,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::{
    connection::ConnectionService,
    models::{
        AlbumListItem, AlbumListRequest, AlbumListResponse, AppBootstrap, ArtistIndexItem,
        ArtistIndexesRequest, ArtistIndexesResponse, ConnectServerProfileRequest,
        ConnectServerProfileResult, CoverArtRequest, CoverArtResponse, MusicDirectoryChild,
        MusicDirectoryRequest, MusicDirectoryResponse, MusicFolderSummary, MusicFoldersResponse,
        ProfileIdRequest,
    },
    secrets::OsKeyringSecretStore,
    session::SessionService,
    ActiveSessionState,
};

#[tauri::command]
pub async fn bootstrap_app_state<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.bootstrap_app_state(&state.0).await
}

#[tauri::command]
pub async fn connect_server_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: ConnectServerProfileRequest,
) -> Result<ConnectServerProfileResult, String> {
    build_service(&app)?
        .connect_server_profile(&state.0, payload)
        .await
}

#[tauri::command]
pub fn delete_server_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: ProfileIdRequest,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.delete_server_profile(&state.0, &payload.profile_id)
}

#[tauri::command]
pub async fn get_album_list<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: AlbumListRequest,
) -> Result<AlbumListResponse, String> {
    let context = payload.context.clone();
    let client = build_service(&app)?.build_active_client(&state.0)?;
    let response = client
        .get_album_list2(payload.to_open_subsonic_request()?)
        .await
        .map_err(format_api_error)?;

    Ok(AlbumListResponse {
        context,
        albums: parse_album_list_items(response.payload.album_list2)?,
    })
}

#[tauri::command]
pub async fn get_music_folders<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
) -> Result<MusicFoldersResponse, String> {
    let client = build_service(&app)?.build_active_client(&state.0)?;
    let response = client
        .get_music_folders(GetMusicFoldersRequest::default())
        .await
        .map_err(format_api_error)?;

    Ok(MusicFoldersResponse {
        music_folders: parse_music_folders(response.payload.music_folders)?,
    })
}

#[tauri::command]
pub async fn get_artist_indexes<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: Option<ArtistIndexesRequest>,
) -> Result<ArtistIndexesResponse, String> {
    let music_folder_id = payload.and_then(|payload| {
        payload.music_folder_id.and_then(|id| {
            let trimmed = id.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
    });
    let client = build_service(&app)?.build_active_client(&state.0)?;
    let response = client
        .get_indexes(GetIndexesRequest {
            music_folder_id,
            if_modified_since: None,
        })
        .await
        .map_err(format_api_error)?;

    Ok(ArtistIndexesResponse {
        artists: parse_artist_indexes(response.payload.indexes)?,
    })
}

#[tauri::command]
pub async fn get_music_directory<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: MusicDirectoryRequest,
) -> Result<MusicDirectoryResponse, String> {
    let directory_id = payload.id.trim();
    if directory_id.is_empty() {
        return Err("id is required.".to_string());
    }

    let client = build_service(&app)?.build_active_client(&state.0)?;
    let response = client
        .get_music_directory(GetMusicDirectoryRequest {
            id: directory_id.to_string(),
        })
        .await
        .map_err(format_api_error)?;

    parse_music_directory(response.payload.directory)
}

#[tauri::command]
pub async fn get_cover_art<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ActiveSessionState>,
    payload: CoverArtRequest,
) -> Result<CoverArtResponse, String> {
    let cover_art_id = payload.cover_art_id.trim();
    if cover_art_id.is_empty() {
        return Err("coverArtId is required.".to_string());
    }

    let client = build_service(&app)?.build_active_client(&state.0)?;
    let request = client
        .get_cover_art(GetCoverArtRequest {
            id: cover_art_id.to_string(),
            size: payload.size,
        })
        .map_err(format_api_error)?;
    let (content_type, bytes) = fetch_binary_response(request).await?;

    Ok(CoverArtResponse {
        data_url: encode_data_url(&content_type, &bytes),
    })
}

fn build_service<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<SessionService<OsKeyringSecretStore, ConnectionService>, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app config directory: {error}"))?;
    let secret_service_name = format!("{}.server-profile", app.config().identifier);
    let api = ConnectionService::new("transonic");

    Ok(SessionService::new(
        config_dir,
        secret_service_name,
        OsKeyringSecretStore,
        api,
    ))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbumListPayload {
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    album: Vec<RawAlbumListItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbumListItem {
    id: String,
    name: String,
    artist: Option<String>,
    cover_art: Option<String>,
    year: Option<u32>,
}

impl From<RawAlbumListItem> for AlbumListItem {
    fn from(value: RawAlbumListItem) -> Self {
        Self {
            id: value.id,
            name: value.name,
            artist: value.artist,
            cover_art_id: value.cover_art,
            year: value.year,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawMusicFoldersPayload {
    #[serde(
        rename = "musicFolder",
        default,
        deserialize_with = "deserialize_vec_or_single"
    )]
    music_folder: Vec<RawMusicFolder>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMusicFolder {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    name: String,
}

impl From<RawMusicFolder> for MusicFolderSummary {
    fn from(value: RawMusicFolder) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndexesPayload {
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    index: Vec<RawIndex>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndex {
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    artist: Vec<RawIndexArtist>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndexArtist {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    name: String,
}

impl From<RawIndexArtist> for ArtistIndexItem {
    fn from(value: RawIndexArtist) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawDirectoryPayload {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    name: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    child: Vec<RawDirectoryChild>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDirectoryChild {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    title: Option<String>,
    name: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    is_dir: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_u32ish")]
    year: Option<u32>,
}

impl From<RawDirectoryChild> for MusicDirectoryChild {
    fn from(value: RawDirectoryChild) -> Self {
        Self {
            id: value.id,
            name: value
                .title
                .or(value.name)
                .or(value.album)
                .unwrap_or_else(|| "Unknown".to_string()),
            artist: value.artist,
            cover_art_id: value.cover_art,
            year: value.year,
            is_directory: value.is_dir.unwrap_or(false),
        }
    }
}

fn deserialize_vec_or_single<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| parse_value(item).map_err(serde::de::Error::custom))
            .collect(),
        Value::Object(_) => Ok(vec![parse_value(value).map_err(serde::de::Error::custom)?]),
        Value::Null => Ok(Vec::new()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected collection payload: {other}"
        ))),
    }
}

fn deserialize_stringish<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::String(value) => Ok(value),
        Value::Number(value) => Ok(value.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected string payload: {other}"
        ))),
    }
}

fn deserialize_optional_boolish<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        Value::Bool(value) => Ok(Some(value)),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Ok(Some(true)),
            "false" | "0" => Ok(Some(false)),
            other => Err(serde::de::Error::custom(format!(
                "unexpected bool payload: {other}"
            ))),
        },
        Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                match value {
                    0 => Ok(Some(false)),
                    1 => Ok(Some(true)),
                    other => Err(serde::de::Error::custom(format!(
                        "unexpected bool payload: {other}"
                    ))),
                }
            } else {
                Err(serde::de::Error::custom(format!(
                    "unexpected bool payload: {value}"
                )))
            }
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected bool payload: {other}"
        ))),
    }
}

fn deserialize_optional_u32ish<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        Value::Number(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("unexpected year payload: {value}"))),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            trimmed.parse::<u32>().map(Some).map_err(|error| {
                serde::de::Error::custom(format!("unexpected year payload: {error}"))
            })
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected year payload: {other}"
        ))),
    }
}

fn parse_album_list_items(payload: Value) -> Result<Vec<AlbumListItem>, String> {
    let payload: RawAlbumListPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the album list payload: {error}"))?;

    Ok(payload.album.into_iter().map(AlbumListItem::from).collect())
}

fn parse_music_folders(payload: Value) -> Result<Vec<MusicFolderSummary>, String> {
    let payload: RawMusicFoldersPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the music folders payload: {error}"))?;

    Ok(payload
        .music_folder
        .into_iter()
        .map(MusicFolderSummary::from)
        .collect())
}

fn parse_artist_indexes(payload: Value) -> Result<Vec<ArtistIndexItem>, String> {
    let payload: RawIndexesPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the indexes payload: {error}"))?;

    Ok(payload
        .index
        .into_iter()
        .flat_map(|index| index.artist)
        .map(ArtistIndexItem::from)
        .collect())
}

fn parse_music_directory(payload: Value) -> Result<MusicDirectoryResponse, String> {
    let payload: RawDirectoryPayload = parse_value(payload)
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

fn parse_value<T>(value: Value) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
}

fn format_api_error(error: ApiError) -> String {
    match error {
        ApiError::InvalidUrl(message)
        | ApiError::ClientBuild(message)
        | ApiError::Protocol(message) => message,
        ApiError::Transport(error) => error.to_string(),
        ApiError::HttpStatus { status, .. } => format!("HTTP {}", status.as_u16()),
        ApiError::Decode { message, .. } => message,
        ApiError::Api { code, message, .. } => {
            let detail = message.unwrap_or_else(|| "Unknown API error".to_string());
            format!("API {code}: {detail}")
        }
        ApiError::UnsupportedExtension { extension } => {
            format!("Missing extension: {extension}")
        }
    }
}

async fn fetch_binary_response(
    request: PreparedBinaryRequest,
) -> Result<(String, Vec<u8>), String> {
    let response = reqwest::Client::new()
        .get(request.url)
        .headers(request.headers)
        .send()
        .await
        .map_err(|error| format!("Failed to fetch binary data: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "The server returned HTTP {} for the binary request.",
            status.as_u16()
        ));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    if !content_type.starts_with("image/") {
        return Err(format!(
            "Unexpected binary content type for cover art: {content_type}"
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Failed to read binary response bytes: {error}"))?
        .to_vec();
    Ok((content_type, bytes))
}

fn encode_data_url(content_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        encode_data_url, parse_album_list_items, parse_artist_indexes, parse_music_directory,
        parse_music_folders,
    };

    #[test]
    fn parse_album_list_items_accepts_array_payloads() {
        let albums = parse_album_list_items(json!({
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
    fn parse_album_list_items_accepts_single_album_objects() {
        let albums = parse_album_list_items(json!({
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

    #[test]
    fn encode_data_url_uses_base64() {
        let encoded = encode_data_url("image/png", b"abc");

        assert_eq!(encoded, "data:image/png;base64,YWJj");
    }

    #[test]
    fn parse_music_folders_accepts_single_folder_objects() {
        let folders = parse_music_folders(json!({
            "musicFolder": {
                "id": 1,
                "name": "music"
            }
        }))
        .unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].id, "1");
        assert_eq!(folders[0].name, "music");
    }

    #[test]
    fn parse_music_folders_accepts_array_payloads() {
        let folders = parse_music_folders(json!({
            "musicFolder": [
                {
                    "id": 1,
                    "name": "music"
                },
                {
                    "id": "4",
                    "name": "upload"
                }
            ]
        }))
        .unwrap();

        assert_eq!(folders.len(), 2);
        assert_eq!(folders[1].id, "4");
        assert_eq!(folders[1].name, "upload");
    }

    #[test]
    fn parse_artist_indexes_accepts_single_artist_objects() {
        let artists = parse_artist_indexes(json!({
            "index": {
                "artist": {
                    "id": "artist-1",
                    "name": "Artist One"
                }
            }
        }))
        .unwrap();

        assert_eq!(artists.len(), 1);
        assert_eq!(artists[0].id, "artist-1");
        assert_eq!(artists[0].name, "Artist One");
    }

    #[test]
    fn parse_artist_indexes_flattens_multiple_index_groups() {
        let artists = parse_artist_indexes(json!({
            "index": [
                {
                    "artist": [
                        {
                            "id": "artist-1",
                            "name": "Artist One"
                        }
                    ]
                },
                {
                    "artist": {
                        "id": "artist-2",
                        "name": "Artist Two"
                    }
                }
            ]
        }))
        .unwrap();

        assert_eq!(artists.len(), 2);
        assert_eq!(artists[1].id, "artist-2");
        assert_eq!(artists[1].name, "Artist Two");
    }

    #[test]
    fn parse_music_directory_keeps_directory_and_file_children() {
        let response = parse_music_directory(json!({
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
        assert_eq!(response.children[0].name, "Album One");
        assert_eq!(
            response.children[0].cover_art_id.as_deref(),
            Some("cover-1")
        );
        assert_eq!(response.children[0].year, Some(2024));
        assert!(response.children[0].is_directory);
        assert_eq!(response.children[1].name, "Song One");
        assert!(!response.children[1].is_directory);
    }
}
