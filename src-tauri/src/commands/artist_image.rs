use tauri::{AppHandle, State};

use crate::{
    models::{ArtistImageRequest, ArtistImageResponse},
    ActiveSessionState, ArtistImageCacheState,
};

#[tauri::command]
#[specta::specta]
pub async fn get_artist_image(
    _app: AppHandle,
    state: State<'_, ActiveSessionState>,
    artist_image_cache: State<'_, ArtistImageCacheState>,
    payload: ArtistImageRequest,
) -> Result<ArtistImageResponse, String> {
    let url = payload.url.trim();
    if url.is_empty() {
        return Err("url is required.".to_string());
    }

    let profile_id = {
        let guard = state.0.lock().unwrap();
        let session = guard
            .as_ref()
            .ok_or_else(|| "No active session is available.".to_string())?;
        session.profile_id.clone()
    };
    let cache = artist_image_cache.0.clone();
    let url = url.to_string();
    let local_path =
        tauri::async_runtime::spawn_blocking(move || cache.resolve_artist_image(&profile_id, &url))
            .await
            .map_err(|error| format!("Failed to join the artist image cache task: {error}"))??;

    Ok(ArtistImageResponse {
        local_path: local_path.to_string_lossy().to_string(),
    })
}
