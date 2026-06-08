use tauri::{AppHandle, State};

use crate::{
    commands::playback::active_runtime_parts,
    models::{effective_transcoding_codec, AppSettings, SettingsUpdateRequest},
    playback::PlaybackRuntimeContext,
    ActiveSessionState, AppSettingsState, CoverArtCacheState, PlaybackControllerState,
};

fn snapshot_settings(state: &AppSettingsState) -> Result<AppSettings, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The app settings state is unavailable.".to_string())?;
    Ok(guard.snapshot().0)
}

#[tauri::command]
#[specta::specta]
pub fn settings_update(
    app: AppHandle,
    settings: State<'_, AppSettingsState>,
    playback: State<'_, PlaybackControllerState>,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    payload: SettingsUpdateRequest,
) -> Result<AppSettings, String> {
    let (previous_settings, updated_settings) = {
        let mut guard = settings
            .0
            .lock()
            .map_err(|_| "The app settings state is unavailable.".to_string())?;
        let previous = guard.snapshot().0;
        let updated = guard.replace(payload.settings)?;
        (previous, updated)
    };

    let previous_transcoding_codec = effective_transcoding_codec(
        previous_settings.playback.use_custom_transcoding_codec,
        previous_settings.playback.transcoding_codec,
    );
    let updated_transcoding_codec = effective_transcoding_codec(
        updated_settings.playback.use_custom_transcoding_codec,
        updated_settings.playback.transcoding_codec,
    );

    if previous_settings.playback.stream_mode != updated_settings.playback.stream_mode
        || previous_settings.playback.transcoding_bitrate_limit
            != updated_settings.playback.transcoding_bitrate_limit
        || previous_transcoding_codec != updated_transcoding_codec
    {
        let mut controller = playback
            .0
            .lock()
            .map_err(|_| "The playback controller state is unavailable.".to_string())?;

        let update_result = if let Ok((client, capability_matrix, profile_id)) =
            active_runtime_parts(&app, &sessions)
        {
            let runtime_context = PlaybackRuntimeContext {
                client: &client,
                capability_matrix: &capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&profile_id),
            };
            controller.set_stream_settings(
                updated_settings.playback.stream_mode,
                updated_settings.playback.transcoding_bitrate_limit,
                updated_transcoding_codec,
                Some(&runtime_context),
            )
        } else {
            controller.set_stream_settings(
                updated_settings.playback.stream_mode,
                updated_settings.playback.transcoding_bitrate_limit,
                updated_transcoding_codec,
                None,
            )
        };

        if let Err(error) = update_result {
            log::warn!("settings_update: failed to refresh stream settings: {error}");
        }
    }

    if previous_settings.playback.gapless_playback_enabled
        != updated_settings.playback.gapless_playback_enabled
    {
        let mut controller = playback
            .0
            .lock()
            .map_err(|_| "The playback controller state is unavailable.".to_string())?;

        let update_result = if let Ok((client, capability_matrix, profile_id)) =
            active_runtime_parts(&app, &sessions)
        {
            let runtime_context = PlaybackRuntimeContext {
                client: &client,
                capability_matrix: &capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&profile_id),
            };
            controller.set_gapless_playback_enabled(
                updated_settings.playback.gapless_playback_enabled,
                Some(&runtime_context),
            )
        } else {
            controller.set_gapless_playback_enabled(
                updated_settings.playback.gapless_playback_enabled,
                None,
            )
        };

        if let Err(error) = update_result {
            log::warn!("settings_update: failed to refresh gapless: {error}");
        }
    }

    if (previous_settings.playback.volume - updated_settings.playback.volume).abs() > f32::EPSILON {
        let mut controller = playback
            .0
            .lock()
            .map_err(|_| "The playback controller state is unavailable.".to_string())?;
        if let Err(error) = controller.set_volume(updated_settings.playback.volume) {
            log::warn!("settings_update: failed to refresh volume: {error}");
        }
    }

    if previous_settings.connect != updated_settings.connect {
        crate::connect::restart(&app);
    }
    snapshot_settings(&settings)
}

#[tauri::command]
#[specta::specta]
pub fn get_default_device_name(_app: AppHandle) -> String {
    #[cfg(target_os = "android")]
    if let Some(name) = crate::playback::android_default_device_name(&_app) {
        return name;
    }

    let hostname = tauri_plugin_os::hostname();
    if !hostname.trim().is_empty() {
        return hostname;
    }

    std::env::consts::OS.to_string()
}
