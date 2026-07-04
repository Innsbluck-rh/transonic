use tauri::{AppHandle, State};

use crate::{
    commands::playback::active_runtime_parts,
    models::{
        effective_output_device_id, normalize_output_device_id, resolve_effective_stream_settings,
        AppSettings, NetworkCostState, SettingsUpdateRequest,
    },
    playback::PlaybackRuntimeContext,
    ActiveSessionState, AppSettingsState, CoverArtCacheState, NetworkCostStateState,
    PlaybackControllerState,
};

fn snapshot_settings(state: &AppSettingsState) -> Result<AppSettings, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The app settings state is unavailable.".to_string())?;
    Ok(guard.snapshot().0)
}

fn snapshot_network_cost_state(state: &NetworkCostStateState) -> Result<NetworkCostState, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The network cost state is unavailable.".to_string())?;
    Ok(*guard)
}

fn format_settings_update_error(error: String, rollback_error: Option<String>) -> String {
    match rollback_error {
        Some(rollback_error) => {
            format!("{error}; failed to roll back persisted settings: {rollback_error}")
        }
        None => error,
    }
}

fn output_device_id_changed(previous: Option<&str>, requested: Option<&str>) -> bool {
    previous != requested
}

#[tauri::command]
#[specta::specta]
pub fn settings_update(
    app: AppHandle,
    settings: State<'_, AppSettingsState>,
    playback: State<'_, PlaybackControllerState>,
    sessions: State<'_, ActiveSessionState>,
    cover_art_cache: State<'_, CoverArtCacheState>,
    network_cost: State<'_, NetworkCostStateState>,
    payload: SettingsUpdateRequest,
) -> Result<AppSettings, String> {
    let mut requested_settings = payload.settings;
    requested_settings.playback.output_device_id =
        normalize_output_device_id(requested_settings.playback.output_device_id);

    let (previous_settings, updated_settings) = {
        let mut guard = settings
            .0
            .lock()
            .map_err(|_| "The app settings state is unavailable.".to_string())?;
        let previous = guard.snapshot().0;
        let previous_output_device_id = effective_output_device_id(&previous.playback);
        let requested_output_device_id = effective_output_device_id(&requested_settings.playback);
        if output_device_id_changed(previous_output_device_id, requested_output_device_id) {
            if let Some(requested_output_device_id) = requested_output_device_id {
                crate::playback::validate_output_device_id(Some(
                    requested_output_device_id.to_string(),
                ))?;
            }
        }
        let updated = guard.replace(requested_settings)?;
        (previous, updated)
    };

    let network_cost_state = snapshot_network_cost_state(&network_cost)?;
    let previous_stream_settings =
        resolve_effective_stream_settings(&previous_settings.playback, network_cost_state);
    let updated_stream_settings =
        resolve_effective_stream_settings(&updated_settings.playback, network_cost_state);

    let previous_output_device_id =
        effective_output_device_id(&previous_settings.playback).map(ToString::to_string);
    let updated_output_device_id =
        effective_output_device_id(&updated_settings.playback).map(ToString::to_string);

    if previous_output_device_id != updated_output_device_id {
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
            controller.set_output_device(updated_output_device_id.clone(), Some(&runtime_context))
        } else {
            controller.set_output_device(updated_output_device_id.clone(), None)
        };

        if let Err(error) = update_result {
            log::warn!("settings_update: failed to refresh output device: {error}");
            drop(controller);
            let rollback_result = settings
                .0
                .lock()
                .map_err(|_| "The app settings state is unavailable.".to_string())
                .and_then(|mut guard| guard.replace(previous_settings.clone()).map(|_| ()));
            return Err(format_settings_update_error(error, rollback_result.err()));
        }
    }

    if previous_stream_settings != updated_stream_settings {
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
                updated_stream_settings.stream_mode,
                updated_stream_settings.transcoding_bitrate_limit,
                updated_stream_settings.transcoding_codec,
                Some(&runtime_context),
            )
        } else {
            controller.set_stream_settings(
                updated_stream_settings.stream_mode,
                updated_stream_settings.transcoding_bitrate_limit,
                updated_stream_settings.transcoding_codec,
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

#[cfg(test)]
mod tests {
    use super::output_device_id_changed;

    #[test]
    fn unchanged_output_device_id_does_not_require_validation() {
        assert!(!output_device_id_changed(
            Some("output:temporarily-missing"),
            Some("output:temporarily-missing")
        ));
        assert!(!output_device_id_changed(None, None));
    }

    #[test]
    fn changed_output_device_id_requires_validation() {
        assert!(output_device_id_changed(None, Some("output:device-1")));
        assert!(output_device_id_changed(
            Some("output:device-1"),
            Some("output:device-2")
        ));
        assert!(output_device_id_changed(Some("output:device-1"), None));
    }
}
