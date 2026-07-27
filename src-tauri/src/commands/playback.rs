use tauri::{AppHandle, Manager, State};

use crate::{
    commands::browse::{
        album::load_album_songs_from_client,
        folder_structure_album_songs::load_album_songs_from_client as load_folder_album_songs_from_client,
    },
    commands::common::{client, format_api_error},
    connect::ConnectState,
    models::{
        CapabilityMatrix, ConnectPlaybackCommand, PlaybackAppendToQueueRequest,
        PlaybackInsertAfterCurrentRequest,
        PlaybackMoveQueueIndexRequest, PlaybackOutputDevice, PlaybackPlayQueueIndexRequest,
        PlaybackRemoveQueueIndexRequest, PlaybackSeekRequest, PlaybackSetPositionRequest,
        PlaybackSetQueueRequest, PlaybackSetVolumeRequest, PlaybackStatus, QueueSource,
        SongResponse,
    },
    playback::PlaybackRuntimeContext,
    ActiveSessionState, CoverArtCacheState, PlaybackControllerState,
};

fn send_connect_playback_command(
    app: &AppHandle,
    command: ConnectPlaybackCommand,
) -> Result<bool, String> {
    app.state::<ConnectState>()
        .0
        .send_playback_command(command)
}

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

pub(crate) fn active_runtime_parts(
    app: &AppHandle,
    sessions: &State<'_, ActiveSessionState>,
) -> Result<
    (
        opensubsonic_client::OpenSubsonicClient,
        CapabilityMatrix,
        String,
    ),
    String,
> {
    let active_capability_matrix = active_capability_matrix(sessions)?;
    let active_profile_id = active_profile_id(sessions)?;
    let active_client = client(app, &sessions.0)?;
    Ok((active_client, active_capability_matrix, active_profile_id))
}

#[cfg(target_os = "android")]
fn spawn_current_artwork_update(app: &AppHandle) {
    crate::playback::spawn_android_artwork_update(app.clone());
}

#[cfg(not(target_os = "android"))]
fn spawn_current_artwork_update(_app: &AppHandle) {}

async fn flatten_queue_sources(
    app: &AppHandle,
    items: Vec<QueueSource>,
) -> Result<Vec<SongResponse>, String> {
    let needs_resolution = items
        .iter()
        .any(|item| !matches!(item, QueueSource::Songs { .. }));

    if !needs_resolution {
        return Ok(items
            .into_iter()
            .flat_map(|item| match item {
                QueueSource::Songs { songs } => songs,
                _ => unreachable!(),
            })
            .collect());
    }

    let active_client = {
        let sessions = app.state::<ActiveSessionState>();
        client(app, &sessions.0)?
    };

    let mut songs = Vec::new();
    for item in items {
        match item {
            QueueSource::Songs { songs: s } => songs.extend(s),
            QueueSource::Album { album_id } => {
                let trimmed = album_id.trim().to_string();
                if trimmed.is_empty() {
                    return Err("albumId is required.".to_string());
                }
                let result = load_album_songs_from_client(&active_client, &trimmed)
                    .await
                    .map_err(format_api_error)?;
                songs.extend(result.songs);
            }
            QueueSource::FolderAlbum {
                library_id,
                node_id,
                album_id,
            } => {
                let lib_id = library_id.trim().to_string();
                let nd_id = node_id.trim().to_string();
                let alb_id = album_id.trim().to_string();
                if lib_id.is_empty() {
                    return Err("libraryId is required.".to_string());
                }
                if nd_id.is_empty() {
                    return Err("nodeId is required.".to_string());
                }
                if alb_id.is_empty() {
                    return Err("albumId is required.".to_string());
                }
                let result =
                    load_folder_album_songs_from_client(&active_client, &lib_id, &nd_id, &alb_id)
                        .await
                        .map_err(format_api_error)?;
                songs.extend(result.songs);
            }
        }
    }
    Ok(songs)
}

#[tauri::command]
#[specta::specta]
pub fn playback_get_state(
    app: AppHandle,
    state: State<'_, PlaybackControllerState>,
) -> Result<PlaybackStatus, String> {
    let mut controller = state
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;

    // Try to build a runtime context so that native events (e.g. track-ended)
    // can trigger auto-advance.  Falls back to context-free polling if the
    // active session is not available.
    let sessions = app.state::<ActiveSessionState>();
    let cover_art_cache = app.state::<CoverArtCacheState>();

    let context_parts = (|| -> Result<_, String> { active_runtime_parts(&app, &sessions) })();

    match context_parts {
        Ok((ref cl, ref cm, ref pid)) => {
            let rc = PlaybackRuntimeContext {
                client: cl,
                capability_matrix: cm,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(pid.as_str()),
            };
            controller.synced_state_with_context(Some(&rc))
        }
        Err(_) => controller.synced_state(),
    }
}

/// ASIO devices are only enumerated when asked for, because enumeration loads
/// every registered ASIO driver in turn: it costs seconds and briefly hands
/// control of unrelated audio hardware to drivers nobody selected.
///
/// This is a query parameter rather than a read of the saved setting on purpose.
/// The caller updates its settings store optimistically and only then awaits the
/// settings command, so reading the stored value here answered from the previous
/// state and the list stayed without ASIO devices until something else forced a
/// refetch. Whether an ASIO device may actually be *played* is a separate
/// question, still decided against the saved setting by
/// `effective_output_device_id`.
#[tauri::command]
#[specta::specta]
pub fn playback_get_output_devices(
    include_asio: bool,
) -> Result<Vec<PlaybackOutputDevice>, String> {
    crate::playback::list_output_devices(include_asio)
}

#[tauri::command]
#[specta::specta]
pub fn playback_set_queue(app: AppHandle, payload: PlaybackSetQueueRequest) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let songs = flatten_queue_sources(&app, payload.items).await?;

            let current_index = payload
                .current_index
                .or_else(|| (!songs.is_empty()).then_some(0));

            if send_connect_playback_command(
                &app,
                ConnectPlaybackCommand {
                    op: "setQueue".to_string(),
                    queue: Some(songs.clone()),
                    current_index,
                    auto_play: payload.auto_play,
                    ..Default::default()
                },
            )? {
                return Ok(());
            }

            {
                let playback = app.state::<PlaybackControllerState>();
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                controller.set_queue(songs, current_index)?;
            }

            if payload.auto_play {
                let sessions = app.state::<ActiveSessionState>();
                let cover_art_cache = app.state::<CoverArtCacheState>();
                let (active_client, active_capability_matrix, active_profile_id) =
                    active_runtime_parts(&app, &sessions)?;
                let runtime_context = PlaybackRuntimeContext {
                    client: &active_client,
                    capability_matrix: &active_capability_matrix,
                    cover_art_cache: Some(&cover_art_cache.0),
                    profile_id: Some(&active_profile_id),
                };

                {
                    let playback = app.state::<PlaybackControllerState>();
                    let mut controller = playback
                        .0
                        .lock()
                        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                    controller.play(&runtime_context)?;
                }
                spawn_current_artwork_update(&app);
            }

            Ok(())
        }
        .await;

        if let Err(error) = result {
            log::error!("playback_set_queue: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_clear_queue(app: AppHandle) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "clearQueue".to_string(),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let runtime_context = active_runtime_parts(&app, &sessions).ok().map(
                |(active_client, active_capability_matrix, active_profile_id)| {
                    (
                        active_client,
                        active_capability_matrix,
                        active_profile_id,
                        cover_art_cache.0.clone(),
                    )
                },
            );

            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            if let Some((
                active_client,
                active_capability_matrix,
                active_profile_id,
                cover_art_cache,
            )) = runtime_context.as_ref()
            {
                let runtime_context = PlaybackRuntimeContext {
                    client: active_client,
                    capability_matrix: active_capability_matrix,
                    cover_art_cache: Some(cover_art_cache),
                    profile_id: Some(active_profile_id),
                };
                controller.clear_queue(Some(&runtime_context))?;
            } else {
                controller.clear_queue(None)?;
            }
            Ok(())
        })();

        if let Err(error) = result {
            log::error!("playback_clear_queue: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_insert_after_current(
    app: AppHandle,
    payload: PlaybackInsertAfterCurrentRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let songs = flatten_queue_sources(&app, payload.items).await?;

            if send_connect_playback_command(
                &app,
                ConnectPlaybackCommand {
                    op: "insertAfterCurrent".to_string(),
                    items: Some(songs.clone()),
                    ..Default::default()
                },
            )? {
                return Ok(());
            }

            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.insert_after_current(songs).map(|_| ())
        }
        .await;

        if let Err(error) = result {
            log::error!("playback_insert_after_current: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_append_to_queue(
    app: AppHandle,
    payload: PlaybackAppendToQueueRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let songs = flatten_queue_sources(&app, payload.items).await?;

            if send_connect_playback_command(
                &app,
                ConnectPlaybackCommand {
                    op: "appendToQueue".to_string(),
                    items: Some(songs.clone()),
                    ..Default::default()
                },
            )? {
                return Ok(());
            }

            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.append_to_queue(songs).map(|_| ())
        }
        .await;

        if let Err(error) = result {
            log::error!("playback_append_to_queue: failed: {error}");
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
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "playQueueIndex".to_string(),
            index: Some(payload.index),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let (active_client, active_capability_matrix, active_profile_id) =
                active_runtime_parts(&app, &sessions)?;
            let runtime_context = PlaybackRuntimeContext {
                client: &active_client,
                capability_matrix: &active_capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&active_profile_id),
            };

            {
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                controller.play_queue_index(&runtime_context, payload.index)?;
            }
            spawn_current_artwork_update(&app);
            Ok(())
        })();

        if let Err(error) = result {
            log::error!("playback_play_queue_index: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_remove_queue_index(
    app: AppHandle,
    payload: PlaybackRemoveQueueIndexRequest,
) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "removeQueueIndex".to_string(),
            index: Some(payload.index),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let runtime_context = active_runtime_parts(&app, &sessions).ok().map(
                |(active_client, active_capability_matrix, active_profile_id)| {
                    (
                        active_client,
                        active_capability_matrix,
                        active_profile_id,
                        cover_art_cache.0.clone(),
                    )
                },
            );

            {
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                if let Some((
                    active_client,
                    active_capability_matrix,
                    active_profile_id,
                    cover_art_cache,
                )) = runtime_context.as_ref()
                {
                    let runtime_context = PlaybackRuntimeContext {
                        client: active_client,
                        capability_matrix: active_capability_matrix,
                        cover_art_cache: Some(cover_art_cache),
                        profile_id: Some(active_profile_id),
                    };
                    controller.remove_queue_index(Some(&runtime_context), payload.index)?;
                } else {
                    controller.remove_queue_index(None, payload.index)?;
                }
            }
            spawn_current_artwork_update(&app);
            Ok(())
        })();

        if let Err(error) = result {
            log::error!("playback_remove_queue_index: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_move_queue_index(
    app: AppHandle,
    payload: PlaybackMoveQueueIndexRequest,
) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "moveQueueIndex".to_string(),
            from_index: Some(payload.from_index),
            to_index: Some(payload.to_index),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.move_queue_index(payload.from_index, payload.to_index)?;
            spawn_current_artwork_update(&app);
            Ok(())
        })();

        if let Err(error) = result {
            log::error!("playback_move_queue_index: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_set_position(
    app: AppHandle,
    payload: PlaybackSetPositionRequest,
) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "seek".to_string(),
            position_ms: Some(payload.position_ms),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let playback = app.state::<PlaybackControllerState>();
            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            controller.set_position(payload.position_ms).map(|_| ())
        })();

        if let Err(error) = result {
            log::error!("playback_set_position: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_set_volume(
    state: State<'_, PlaybackControllerState>,
    payload: PlaybackSetVolumeRequest,
) -> Result<(), String> {
    let mut controller = state
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    controller.set_volume(payload.volume)
}

#[tauri::command]
#[specta::specta]
pub fn playback_play(app: AppHandle) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "play".to_string(),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let (active_client, active_capability_matrix, active_profile_id) =
                active_runtime_parts(&app, &sessions)?;
            let runtime_context = PlaybackRuntimeContext {
                client: &active_client,
                capability_matrix: &active_capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&active_profile_id),
            };

            {
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                controller.play(&runtime_context)?;
            }
            spawn_current_artwork_update(&app);
            Ok(())
        })();

        if let Err(error) = result {
            log::error!("playback_play: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_pause(app: AppHandle) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "pause".to_string(),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let runtime_context = active_runtime_parts(&app, &sessions).ok().map(
                |(active_client, active_capability_matrix, active_profile_id)| {
                    (
                        active_client,
                        active_capability_matrix,
                        active_profile_id,
                        cover_art_cache.0.clone(),
                    )
                },
            );

            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            if let Some((
                active_client,
                active_capability_matrix,
                active_profile_id,
                cover_art_cache,
            )) = runtime_context.as_ref()
            {
                let runtime_context = PlaybackRuntimeContext {
                    client: active_client,
                    capability_matrix: active_capability_matrix,
                    cover_art_cache: Some(cover_art_cache),
                    profile_id: Some(active_profile_id),
                };
                controller
                    .pause_with_context(Some(&runtime_context))
                    .map(|_| ())
            } else {
                controller.pause().map(|_| ())
            }
        })();

        if let Err(error) = result {
            log::error!("playback_pause: failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn playback_stop(app: AppHandle) -> Result<(), String> {
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "stop".to_string(),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let runtime_context = active_runtime_parts(&app, &sessions).ok().map(
                |(active_client, active_capability_matrix, active_profile_id)| {
                    (
                        active_client,
                        active_capability_matrix,
                        active_profile_id,
                        cover_art_cache.0.clone(),
                    )
                },
            );

            let mut controller = playback
                .0
                .lock()
                .map_err(|_| "The playback controller state is unavailable.".to_string())?;
            if let Some((
                active_client,
                active_capability_matrix,
                active_profile_id,
                cover_art_cache,
            )) = runtime_context.as_ref()
            {
                let runtime_context = PlaybackRuntimeContext {
                    client: active_client,
                    capability_matrix: active_capability_matrix,
                    cover_art_cache: Some(cover_art_cache),
                    profile_id: Some(active_profile_id),
                };
                controller
                    .stop_with_context(Some(&runtime_context))
                    .map(|_| ())
            } else {
                controller.stop().map(|_| ())
            }
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
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "seek".to_string(),
            position_ms: Some(payload.position_ms),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let (active_client, active_capability_matrix, active_profile_id) =
                active_runtime_parts(&app, &sessions)?;
            let runtime_context = PlaybackRuntimeContext {
                client: &active_client,
                capability_matrix: &active_capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&active_profile_id),
            };

            {
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                controller.seek(&runtime_context, payload.position_ms)?;
            }
            spawn_current_artwork_update(&app);
            Ok(())
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
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "next".to_string(),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let (active_client, active_capability_matrix, active_profile_id) =
                active_runtime_parts(&app, &sessions)?;
            let runtime_context = PlaybackRuntimeContext {
                client: &active_client,
                capability_matrix: &active_capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&active_profile_id),
            };

            {
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                controller.next(&runtime_context)?;
            }
            spawn_current_artwork_update(&app);
            Ok(())
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
    if send_connect_playback_command(
        &app,
        ConnectPlaybackCommand {
            op: "prev".to_string(),
            ..Default::default()
        },
    )? {
        return Ok(());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| -> Result<(), String> {
            let sessions = app.state::<ActiveSessionState>();
            let cover_art_cache = app.state::<CoverArtCacheState>();
            let playback = app.state::<PlaybackControllerState>();

            let (active_client, active_capability_matrix, active_profile_id) =
                active_runtime_parts(&app, &sessions)?;
            let runtime_context = PlaybackRuntimeContext {
                client: &active_client,
                capability_matrix: &active_capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&active_profile_id),
            };

            {
                let mut controller = playback
                    .0
                    .lock()
                    .map_err(|_| "The playback controller state is unavailable.".to_string())?;
                controller.prev(&runtime_context)?;
            }
            spawn_current_artwork_update(&app);
            Ok(())
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
