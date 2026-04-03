use tauri::{AppHandle, State};

use crate::{
    commands::browse::{
        album::load_album_songs_from_client,
        folder_structure_album_songs::load_album_songs_from_client as load_folder_album_songs_from_client,
    },
    commands::common::{client, format_api_error},
    models::{
        AlbumSongsRequest, CapabilityMatrix, FolderStructureAlbumSongsRequest,
        PlaybackPlayAlbumRequest, PlaybackPlayFolderAlbumRequest, PlaybackPlayQueueIndexRequest,
        PlaybackPlaySongsRequest, PlaybackQueueEntry, PlaybackSeekRequest, PlaybackSetQueueRequest,
        PlaybackStatus, SongResponse,
    },
    playback::PlaybackRuntimeContext,
    ActiveSessionState, CoverArtCacheState, PlaybackControllerState,
};

fn active_capability_matrix(state: &ActiveSessionState) -> Result<CapabilityMatrix, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The active session state is unavailable.".to_string())?;
    let session = guard
        .as_ref()
        .ok_or_else(|| "No active session is available.".to_string())?;
    Ok(session.capability_matrix.clone())
}

fn active_profile_id(state: &ActiveSessionState) -> Result<String, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The active session state is unavailable.".to_string())?;
    let session = guard
        .as_ref()
        .ok_or_else(|| "No active session is available.".to_string())?;
    Ok(session.profile_id.clone())
}

#[tauri::command]
#[specta::specta]
pub fn playback_get_state(
    state: State<'_, PlaybackControllerState>,
) -> Result<PlaybackStatus, String> {
    let mut controller = state
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    controller.synced_state()
}

#[tauri::command]
#[specta::specta]
pub fn playback_set_queue(
    state: State<'_, PlaybackControllerState>,
    payload: PlaybackSetQueueRequest,
) -> Result<(), String> {
    let mut controller = state
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller.set_queue(payload).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_set_queue: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_play_queue_index(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: PlaybackPlayQueueIndexRequest,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller
        .play_queue_index(&runtime_context, payload.index)
        .map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_play_queue_index: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_play(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller.play(&runtime_context).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_play: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn playback_play_album(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: PlaybackPlayAlbumRequest,
) -> Result<(), String> {
    let album_id = AlbumSongsRequest {
        id: payload.album_id,
    };
    let trimmed_album_id = album_id.id.trim();
    if trimmed_album_id.is_empty() {
        return Err("albumId is required.".to_string());
    }

    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let songs = load_album_songs_from_client(&active_client, trimmed_album_id)
        .await
        .map_err(format_api_error)?
        .songs;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = replace_queue_and_play(&mut controller, &runtime_context, songs, None).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_play_album: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn playback_play_folder_album(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: PlaybackPlayFolderAlbumRequest,
) -> Result<(), String> {
    let request = FolderStructureAlbumSongsRequest {
        library_id: payload.library_id,
        node_id: payload.node_id,
        album_id: payload.album_id,
    };
    let library_id = request.library_id.trim();
    if library_id.is_empty() {
        return Err("libraryId is required.".to_string());
    }

    let node_id = request.node_id.trim();
    if node_id.is_empty() {
        return Err("nodeId is required.".to_string());
    }

    let album_id = request.album_id.trim();
    if album_id.is_empty() {
        return Err("albumId is required.".to_string());
    }

    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let songs = load_folder_album_songs_from_client(&active_client, library_id, node_id, album_id)
        .await
        .map_err(format_api_error)?
        .songs;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = replace_queue_and_play(&mut controller, &runtime_context, songs, None).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_play_folder_album: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_pause(state: State<'_, PlaybackControllerState>) -> Result<(), String> {
    let mut controller = state
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller.pause().map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_pause: failed: {error}");
    }
    result
}

fn queue_entries_from_songs(songs: Vec<SongResponse>) -> Vec<PlaybackQueueEntry> {
    songs.into_iter().map(PlaybackQueueEntry::from).collect()
}

#[tauri::command]
#[specta::specta]
pub fn playback_play_songs(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: PlaybackPlaySongsRequest,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = replace_queue_and_play(
        &mut controller,
        &runtime_context,
        payload.songs,
        Some(payload.start_index),
    )
    .map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_play_songs: failed: {error}");
    }
    result
}

fn replace_queue_and_play(
    controller: &mut crate::playback::PlaybackController,
    runtime_context: &PlaybackRuntimeContext<'_>,
    songs: Vec<SongResponse>,
    start_index: Option<u32>,
) -> Result<PlaybackStatus, String> {
    let entries = queue_entries_from_songs(songs);
    let current_index = start_index.or_else(|| (!entries.is_empty()).then_some(0));

    controller.set_queue(PlaybackSetQueueRequest {
        entries,
        current_index,
    })?;
    controller.play(runtime_context)
}

#[cfg(test)]
mod tests {
    use crate::models::SongResponse;

    use super::queue_entries_from_songs;

    fn song(id: &str) -> SongResponse {
        SongResponse {
            id: id.to_string(),
            parent_id: None,
            path: Some(format!("/music/{id}.mp3")),
            title: format!("Title {id}"),
            album: Some("Album".to_string()),
            album_id: Some("album-1".to_string()),
            artist: Some("Artist".to_string()),
            artist_id: Some("artist-1".to_string()),
            cover_art_id: Some("cover-1".to_string()),
            track: Some(1),
            disc_number: Some(1),
            year: Some(2024),
            duration: Some(180),
            size: None,
            content_type: None,
            suffix: None,
            bit_rate: None,
            genre: None,
            created: None,
            starred: None,
            is_directory: false,
            media_type: Some("song".to_string()),
        }
    }

    #[test]
    fn queue_entries_from_songs_copies_artist_and_album_ids() {
        let entries = queue_entries_from_songs(vec![song("song-1")]);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].song_id, "song-1");
        assert_eq!(entries[0].artist_id.as_deref(), Some("artist-1"));
        assert_eq!(entries[0].album_id.as_deref(), Some("album-1"));
    }
}

#[tauri::command]
#[specta::specta]
pub fn playback_stop(state: State<'_, PlaybackControllerState>) -> Result<(), String> {
    let mut controller = state
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller.stop().map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_stop: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_seek(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
    payload: PlaybackSeekRequest,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller
        .seek(&runtime_context, payload.position_ms)
        .map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_seek: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_next(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller.next(&runtime_context).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_next: failed: {error}");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub fn playback_prev(
    app: AppHandle,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    playback: State<'_, PlaybackControllerState>,
) -> Result<(), String> {
    let active_capability_matrix = active_capability_matrix(&sessions)?;
    let active_profile_id = active_profile_id(&sessions)?;
    let active_client = client(&app, &sessions.0)?;
    let runtime_context = PlaybackRuntimeContext {
        client: &active_client,
        capability_matrix: &active_capability_matrix,
        cover_art_cache: Some(&cover_art_cache.0),
        profile_id: Some(&active_profile_id),
    };

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let result = controller.prev(&runtime_context).map(|_| ());
    if let Err(error) = &result {
        log::error!("playback_prev: failed: {error}");
    }
    result
}
