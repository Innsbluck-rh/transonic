mod backend_shims;
mod controller;
mod native_events;
mod queue_sync;
mod reporting;
mod server_reporting;
#[cfg(all(target_os = "windows", not(test)))]
mod windows_runtime;
use crate::models::{normalize_output_device_id, PlaybackOutputDevice};
#[cfg(not(test))]
use crate::playback_state::PlaybackStatePersister;

#[cfg(target_os = "android")]
mod android_mobile_plugin;
#[cfg(target_os = "android")]
mod android_runtime;

#[cfg(not(test))]
use queue_sync::NoopQueueSyncGateway;
#[cfg(not(test))]
use reporting::{CompositePlaybackReporter, TauriPlaybackReporter};
#[cfg(not(test))]
use server_reporting::BackgroundPlaybackServerReporter;

#[cfg(not(any(target_os = "android", target_os = "windows")))]
use crate::playback::native_events::NoopNativePlaybackEventSource;

#[cfg(target_os = "android")]
pub(crate) use android_mobile_plugin::init as init_android_mobile_plugin;
#[cfg(target_os = "android")]
pub(crate) use android_runtime::consume_pending_notification_tap;
#[cfg(target_os = "android")]
pub(crate) use android_runtime::install_android_app_handle;
#[cfg(target_os = "android")]
pub(crate) use android_runtime::spawn_android_artwork_update;
pub use controller::{PlaybackController, PlaybackRuntimeContext};
#[cfg(not(test))]
pub use reporting::PlaybackEventAppHandle;
pub(crate) use reporting::PlaybackReporter;
#[cfg(all(target_os = "windows", not(test)))]
pub(crate) use windows_runtime::install_windows_app_handle;
#[cfg(all(target_os = "windows", not(test)))]
pub(crate) use windows_runtime::{
    spawn_controller_process_native_events, spawn_controller_process_native_events_after,
};

#[cfg(any(not(target_os = "windows"), test))]
pub(crate) fn spawn_controller_process_native_events_after(_delay: std::time::Duration) {}

#[cfg(test)]
pub(crate) fn spawn_controller_process_native_events() {}

#[cfg(target_os = "android")]
pub(crate) fn android_default_device_name(app: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager;

    app.try_state::<android_mobile_plugin::AndroidPlaybackRuntimeState>()
        .and_then(|state| state.bridge.default_device_name().ok())
        .filter(|name| !name.trim().is_empty())
}

#[cfg(target_os = "android")]
pub(crate) fn start_android_network_cost_monitoring(app: &tauri::AppHandle) {
    use tauri::Manager;

    let Some(runtime_state) = app.try_state::<android_mobile_plugin::AndroidPlaybackRuntimeState>()
    else {
        return;
    };
    match runtime_state.bridge.start_network_cost_monitoring() {
        Ok(state) => {
            if let Err(error) = android_runtime::apply_network_cost_state_str(app, &state) {
                log::warn!("android: failed to apply initial network cost state: {error}");
            }
        }
        Err(error) => {
            log::warn!("android: failed to start network cost monitoring: {error}");
        }
    }
}

#[cfg(not(test))]
pub(crate) fn create_playback_controller(
    _app: &tauri::AppHandle,
    app_handle: PlaybackEventAppHandle,
    persister: Box<dyn PlaybackStatePersister>,
    gapless_playback_enabled: bool,
    initial_volume: f32,
    initial_output_device_id: Option<String>,
) -> PlaybackController {
    let reporter: Box<dyn reporting::PlaybackReporter> =
        Box::new(CompositePlaybackReporter::new(vec![
            Box::new(TauriPlaybackReporter::new(app_handle)),
            Box::new(crate::connect::ConnectPlaybackReporter::from_app(_app)),
        ]));

    #[cfg(target_os = "android")]
    {
        use tauri::Manager;

        if let Some(runtime_state) =
            _app.try_state::<android_mobile_plugin::AndroidPlaybackRuntimeState>()
        {
            return PlaybackController::new(
                backend_shims::create_playback_backend(runtime_state.bridge.clone()),
                reporter,
                Box::new(BackgroundPlaybackServerReporter::new()),
                Box::new(NoopQueueSyncGateway),
                Box::new(runtime_state.event_hub.clone()),
                persister,
                gapless_playback_enabled,
                initial_volume,
                initial_output_device_id,
            );
        }
    }

    #[cfg(target_os = "windows")]
    {
        let event_hub = backend_shims::SymphoniaPlaybackEventHub::default();
        return PlaybackController::new(
            backend_shims::create_playback_backend(event_hub.clone()),
            reporter,
            Box::new(BackgroundPlaybackServerReporter::new()),
            Box::new(NoopQueueSyncGateway),
            Box::new(event_hub),
            persister,
            gapless_playback_enabled,
            initial_volume,
            initial_output_device_id,
        );
    }

    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        return PlaybackController::new(
            backend_shims::create_playback_backend(),
            reporter,
            Box::new(BackgroundPlaybackServerReporter::new()),
            Box::new(NoopQueueSyncGateway),
            Box::new(NoopNativePlaybackEventSource),
            persister,
            gapless_playback_enabled,
            initial_volume,
            initial_output_device_id,
        );
    }

    #[cfg(target_os = "android")]
    panic!("Android playback runtime state was not registered before controller setup.")
}

pub(crate) fn list_output_devices() -> Result<Vec<PlaybackOutputDevice>, String> {
    #[cfg(target_os = "windows")]
    {
        return backend_shims::list_output_devices();
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

pub(crate) fn validate_output_device_id(
    output_device_id: Option<String>,
) -> Result<Option<String>, String> {
    let output_device_id = normalize_output_device_id(output_device_id);

    #[cfg(target_os = "windows")]
    {
        if let Some(id) = output_device_id.as_deref() {
            backend_shims::validate_output_device_id(id)?;
        }
        return Ok(output_device_id);
    }

    #[cfg(not(target_os = "windows"))]
    {
        if output_device_id.is_some() {
            return Err("Output device selection is only supported on Windows.".to_string());
        }
        Ok(output_device_id)
    }
}
