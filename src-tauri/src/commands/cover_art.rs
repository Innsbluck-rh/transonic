use tauri::{AppHandle, State};

use crate::{
    commands::common::client,
    models::{CoverArtRequest, CoverArtResponse},
    ActiveSessionState, CoverArtCacheState,
};

#[tauri::command]
#[specta::specta]
pub async fn get_cover_art(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    payload: CoverArtRequest,
) -> Result<CoverArtResponse, String> {
    let cover_art_id = payload.cover_art_id.trim();
    if cover_art_id.is_empty() {
        return Err("coverArtId is required.".to_string());
    }

    let profile_id = {
        let guard = state.0.lock().unwrap();
        let session = guard
            .as_ref()
            .ok_or_else(|| "No active session is available.".to_string())?;
        session.profile_id.clone()
    };
    let client = client(&app, &state.0)?;
    let cache = cover_art_cache.0.clone();
    let cover_art_id = cover_art_id.to_string();
    let size = payload.size;
    let local_path = tauri::async_runtime::spawn_blocking(move || {
        cache.resolve_cover_art(&client, &profile_id, &cover_art_id, size)
    })
    .await
    .map_err(|error| format!("Failed to join the cover art cache task: {error}"))??;

    Ok(CoverArtResponse {
        local_path: local_path.to_string_lossy().to_string(),
    })
}
