use tauri::{AppHandle, State};

use crate::{
    commands::common::service,
    models::{ActiveSession, CoverArtCacheStatus, CoverArtRequest, CoverArtResponse},
    ActiveSessionState, CoverArtCacheState,
};

struct NormalizedCoverArtRequest {
    session: ActiveSession,
    profile_id: String,
    cover_art_id: String,
    size: Option<u32>,
    cached_fallback_sizes: Vec<u32>,
}

fn normalize_cover_art_request(
    state: &ActiveSessionState,
    payload: CoverArtRequest,
) -> Result<NormalizedCoverArtRequest, String> {
    let profile_id = payload.profile_id.trim();
    if profile_id.is_empty() {
        return Err("profileId is required.".to_string());
    }
    let cover_art_id = payload.cover_art_id.trim();
    if cover_art_id.is_empty() {
        return Err("coverArtId is required.".to_string());
    }

    let session = {
        let guard = state
            .0
            .lock()
            .map_err(|_| "The active session state is unavailable.".to_string())?;
        let session = guard
            .as_ref()
            .ok_or_else(|| "No active session is available.".to_string())?;
        session.clone()
    };
    if session.profile_id != profile_id {
        return Err("The requested profile is no longer active.".to_string());
    }

    Ok(NormalizedCoverArtRequest {
        session,
        profile_id: profile_id.to_string(),
        cover_art_id: cover_art_id.to_string(),
        size: payload.size,
        cached_fallback_sizes: payload.cached_fallback_sizes,
    })
}

fn cover_art_response(local_path: std::path::PathBuf) -> CoverArtResponse {
    CoverArtResponse {
        local_path: local_path.to_string_lossy().to_string(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_cover_art(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    payload: CoverArtRequest,
) -> Result<CoverArtResponse, String> {
    let request = normalize_cover_art_request(&state, payload)?;
    let cache = cover_art_cache.0.clone();
    let cached_profile_id = request.profile_id.clone();
    let cached_cover_art_id = request.cover_art_id.clone();
    let cached_size = request.size;
    let cached_fallback_sizes = request.cached_fallback_sizes.clone();
    let cached_path = tauri::async_runtime::spawn_blocking(move || {
        cache.resolve_cached_cover_art(
            &cached_profile_id,
            &cached_cover_art_id,
            cached_size,
            &cached_fallback_sizes,
        )
    })
    .await
    .map_err(|error| format!("Failed to join the cover art cache task: {error}"))??;
    if let Some(local_path) = cached_path {
        return Ok(cover_art_response(local_path));
    }

    let client = service(&app)?.build_client_from_session(&request.session)?;
    let cache = cover_art_cache.0.clone();
    let cover_art_id = request.cover_art_id;
    let profile_id = request.profile_id;
    let size = request.size;
    let local_path = tauri::async_runtime::spawn_blocking(move || {
        cache.resolve_cover_art(&client, &profile_id, &cover_art_id, size)
    })
    .await
    .map_err(|error| format!("Failed to join the cover art cache task: {error}"))??;

    Ok(cover_art_response(local_path))
}

#[tauri::command]
#[specta::specta]
pub async fn get_cover_art_cached(
    state: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    payload: CoverArtRequest,
) -> Result<Option<CoverArtResponse>, String> {
    let request = normalize_cover_art_request(&state, payload)?;
    let cache = cover_art_cache.0.clone();
    let local_path = tauri::async_runtime::spawn_blocking(move || {
        cache.resolve_cached_cover_art(
            &request.profile_id,
            &request.cover_art_id,
            request.size,
            &request.cached_fallback_sizes,
        )
    })
    .await
    .map_err(|error| format!("Failed to join the cover art cache task: {error}"))??;

    Ok(local_path.map(cover_art_response))
}

fn active_profile_id(state: &ActiveSessionState) -> Result<Option<String>, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The active session state is unavailable.".to_string())?;
    Ok(guard.as_ref().map(|session| session.profile_id.clone()))
}

#[tauri::command]
#[specta::specta]
pub fn get_cover_art_cache_status(
    state: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
) -> Result<CoverArtCacheStatus, String> {
    let profile_id = active_profile_id(&state)?;
    cover_art_cache.0.status(profile_id.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn clear_cover_art_cache(
    state: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
) -> Result<CoverArtCacheStatus, String> {
    let profile_id =
        active_profile_id(&state)?.ok_or_else(|| "No active session is available.".to_string())?;
    cover_art_cache.0.remove_profile(&profile_id)?;
    cover_art_cache.0.status(Some(&profile_id))
}
