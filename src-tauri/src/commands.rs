use tauri::{AppHandle, Manager, State};

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

use crate::{
    browse_parser::{
        parse_album_list_items, parse_artist_indexes, parse_music_directory, parse_music_folders,
    },
    connection::ConnectionService,
    folder_structure::{load_folder_structure_albums, load_folder_structure_roots},
    models::{
        AlbumListRequest, AlbumListResponse, AppBootstrap, ArtistIndexesRequest,
        ArtistIndexesResponse, ConnectServerProfileRequest, ConnectServerProfileResult,
        CoverArtRequest, CoverArtResponse, FolderStructureAlbumsRequest,
        FolderStructureAlbumsResponse, FolderStructureRootsRequest, FolderStructureRootsResponse,
        MusicDirectoryRequest, MusicDirectoryResponse, MusicFoldersResponse, ProfileIdRequest,
    },
    secrets::OsKeyringSecretStore,
    session::SessionService,
    ActiveSessionState,
};

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_app_state(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.bootstrap_app_state(&state.0).await
}

#[tauri::command]
#[specta::specta]
pub async fn connect_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: ConnectServerProfileRequest,
) -> Result<ConnectServerProfileResult, String> {
    build_service(&app)?
        .connect_server_profile(&state.0, payload)
        .await
}

#[tauri::command]
#[specta::specta]
pub fn delete_server_profile(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: ProfileIdRequest,
) -> Result<AppBootstrap, String> {
    build_service(&app)?.delete_server_profile(&state.0, &payload.profile_id)
}

#[tauri::command]
#[specta::specta]
pub async fn get_album_list(
    app: AppHandle,
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
#[specta::specta]
pub async fn get_music_folders(
    app: AppHandle,
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
#[specta::specta]
pub async fn get_folder_structure_roots(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: FolderStructureRootsRequest,
) -> Result<FolderStructureRootsResponse, String> {
    let library_id = payload.library_id.trim();
    if library_id.is_empty() {
        return Err("libraryId is required.".to_string());
    }

    let client = build_service(&app)?.build_active_client(&state.0)?;
    let music_folders = client
        .get_music_folders(GetMusicFoldersRequest::default())
        .await
        .map_err(format_api_error)?;
    let music_folders = parse_music_folders(music_folders.payload.music_folders)?;
    let library = music_folders
        .into_iter()
        .find(|folder| folder.id == library_id)
        .ok_or_else(|| format!("Music folder not found: {library_id}"))?;

    load_folder_structure_roots(&client, &library)
        .await
        .map_err(format_api_error)
}

#[tauri::command]
#[specta::specta]
pub async fn get_artist_indexes(
    app: AppHandle,
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
    let response = BrowsingApi::get_indexes(
        &client,
        GetIndexesRequest {
            music_folder_id,
            if_modified_since: None,
        },
    )
    .await
    .map_err(format_api_error)?;

    Ok(ArtistIndexesResponse {
        artists: parse_artist_indexes(response.payload.indexes)?,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_folder_structure_albums(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: FolderStructureAlbumsRequest,
) -> Result<FolderStructureAlbumsResponse, String> {
    let library_id = payload.library_id.trim();
    if library_id.is_empty() {
        return Err("libraryId is required.".to_string());
    }

    let node_id = payload.node_id.trim();
    if node_id.is_empty() {
        return Err("nodeId is required.".to_string());
    }

    let client = build_service(&app)?.build_active_client(&state.0)?;
    load_folder_structure_albums(&client, library_id, node_id)
        .await
        .map_err(format_api_error)
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

    let client = build_service(&app)?.build_active_client(&state.0)?;
    let response = BrowsingApi::get_music_directory(
        &client,
        GetMusicDirectoryRequest {
            id: directory_id.to_string(),
        },
    )
    .await
    .map_err(format_api_error)?;

    parse_music_directory(response.payload.directory)
}

#[tauri::command]
#[specta::specta]
pub async fn get_cover_art(
    app: AppHandle,
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

fn build_service(
    app: &AppHandle,
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
    use super::encode_data_url;

    #[test]
    fn encode_data_url_uses_base64() {
        let encoded = encode_data_url("image/png", b"abc");

        assert_eq!(encoded, "data:image/png;base64,YWJj");
    }
}
