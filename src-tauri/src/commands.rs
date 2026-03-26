use tauri::{AppHandle, Manager, Runtime, State};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use opensubsonic_client::{
    api::{
        lists::ListsApi,
        retrieval::{GetCoverArtRequest, RetrievalApi},
    },
    ApiError, PreparedBinaryRequest,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    connection::ConnectionService,
    models::{
        AlbumListItem, AlbumListRequest, AlbumListResponse, AppBootstrap,
        ConnectServerProfileRequest, ConnectServerProfileResult, CoverArtRequest, CoverArtResponse,
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
    #[serde(default, deserialize_with = "deserialize_raw_album_items")]
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

fn deserialize_raw_album_items<'de, D>(deserializer: D) -> Result<Vec<RawAlbumListItem>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| serde_json::from_value(item).map_err(serde::de::Error::custom))
            .collect(),
        Value::Object(_) => Ok(vec![
            serde_json::from_value(value).map_err(serde::de::Error::custom)?
        ]),
        Value::Null => Ok(Vec::new()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected album payload: {other}"
        ))),
    }
}

fn parse_album_list_items(payload: Value) -> Result<Vec<AlbumListItem>, String> {
    let payload: RawAlbumListPayload = serde_json::from_value(payload)
        .map_err(|error| format!("Failed to parse the album list payload: {error}"))?;

    Ok(payload.album.into_iter().map(AlbumListItem::from).collect())
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

    use super::{encode_data_url, parse_album_list_items};

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
}
