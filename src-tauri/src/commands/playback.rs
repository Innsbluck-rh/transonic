use tauri::{AppHandle, Manager, State};

use crate::{
    commands::browse::{
        album::load_album_songs_from_client,
        folder_structure_album_songs::load_album_songs_from_client as load_folder_album_songs_from_client,
    },
    commands::common::{client, format_api_error},
    models::{
        CapabilityMatrix, PlaybackPlayAlbumRequest, PlaybackPlayFolderAlbumRequest,
        PlaybackPlayQueueIndexRequest, PlaybackPlaySongsRequest, PlaybackSeekRequest,
        PlaybackSetQueueRequest, PlaybackStatus, SongResponse,
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
pub fn playback_set_queue(app: AppHandle, payload: PlaybackSetQueueRequest) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.set_queue(payload).map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_set_queue: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_play_queue_index(
    app: AppHandle,
    payload: PlaybackPlayQueueIndexRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

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
            controller
                .play_queue_index(&runtime_context, payload.index)
                .map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_play_queue_index: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_play(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

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
            controller.play(&runtime_context).map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_play: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_play_album(
    app: AppHandle,
    payload: PlaybackPlayAlbumRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let trimmed_album_id = payload.album_id.trim().to_string();
            if trimmed_album_id.is_empty() {
                return Err("albumId is required.".to_string());
            }

            let (active_capability_matrix, active_profile_id, active_client) = {
                let sessions = app.state::<ActiveSessionState>();
                let cap = active_capability_matrix(&sessions)?;
                let pid = active_profile_id(&sessions)?;
                let cli = client(&app, &sessions.0)?;
                (cap, pid, cli)
            };

            let songs = load_album_songs_from_client(&active_client, &trimmed_album_id)
                .await
                .map_err(format_api_error)?
                .songs;

            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();
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
            replace_queue_and_play(&mut controller, &runtime_context, songs, None).map(|_| ())
        }
        .await;

        if let Err(error) = result {
            log::error!("playback_play_album: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_play_folder_album(
    app: AppHandle,
    payload: PlaybackPlayFolderAlbumRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let library_id = payload.library_id.trim().to_string();
            if library_id.is_empty() {
                return Err("libraryId is required.".to_string());
            }

            let node_id = payload.node_id.trim().to_string();
            if node_id.is_empty() {
                return Err("nodeId is required.".to_string());
            }

            let album_id = payload.album_id.trim().to_string();
            if album_id.is_empty() {
                return Err("albumId is required.".to_string());
            }

            let (active_capability_matrix, active_profile_id, active_client) = {
                let sessions = app.state::<ActiveSessionState>();
                let cap = active_capability_matrix(&sessions)?;
                let pid = active_profile_id(&sessions)?;
                let cli = client(&app, &sessions.0)?;
                (cap, pid, cli)
            };

            let songs = load_folder_album_songs_from_client(
                &active_client,
                &library_id,
                &node_id,
                &album_id,
            )
            .await
            .map_err(format_api_error)?
            .songs;

            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();
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
            replace_queue_and_play(&mut controller, &runtime_context, songs, None).map(|_| ())
        }
        .await;

        if let Err(error) = result {
            log::error!("playback_play_folder_album: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_pause(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.pause().map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_pause: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_play_songs(
    app: AppHandle,
    payload: PlaybackPlaySongsRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

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
            replace_queue_and_play(
                &mut controller,
                &runtime_context,
                payload.songs,
                Some(payload.start_index),
            )
            .map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_play_songs: failed: {error}");
        }
    });
    Ok(())
}

fn replace_queue_and_play(
    controller: &mut crate::playback::PlaybackController,
    runtime_context: &PlaybackRuntimeContext<'_>,
    songs: Vec<SongResponse>,
    start_index: Option<u32>,
) -> Result<PlaybackStatus, String> {
    let current_index = start_index.or_else(|| (!songs.is_empty()).then_some(0));

    controller.set_queue(PlaybackSetQueueRequest {
        entries: songs,
        current_index,
    })?;
    controller.play(runtime_context)
}

#[tauri::command]
#[specta::specta]
pub fn playback_stop(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.stop().map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_stop: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_seek(app: AppHandle, payload: PlaybackSeekRequest) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

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
            controller
                .seek(&runtime_context, payload.position_ms)
                .map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_seek: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_next(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

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
            controller.next(&runtime_context).map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_next: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_prev(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

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
            controller.prev(&runtime_context).map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_prev: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn consume_pending_notification_tap() -> bool {
    #[cfg(target_os = "android")]
    {
        return crate::playback::consume_pending_notification_tap();
    }
    #[cfg(not(target_os = "android"))]
    {
        return false;
    }
}
