mod backend_shims;
mod controller;
mod native_events;
mod queue_sync;
mod reporting;
mod server_reporting;
#[cfg(target_os = "windows")]
mod windows_runtime;
use crate::playback_state::PlaybackStatePersister;

#[cfg(target_os = "android")]
mod android_mobile_plugin;
#[cfg(target_os = "android")]
mod android_runtime;

use queue_sync::NoopQueueSyncGateway;
use reporting::{CompositePlaybackReporter, TauriPlaybackReporter};
use server_reporting::BackgroundPlaybackServerReporter;

#[cfg(not(any(target_os = "android", target_os = "windows")))]
use crate::playback::native_events::NoopNativePlaybackEventSource;

#[cfg(target_os = "android")]
pub(crate) use android_mobile_plugin::init as init_android_mobile_plugin;
#[cfg(target_os = "android")]
pub(crate) use android_runtime::consume_pending_notification_tap;
#[cfg(target_os = "android")]
pub(crate) use android_runtime::install_android_app_handle;
pub use controller::{PlaybackController, PlaybackRuntimeContext};
pub use reporting::PlaybackEventAppHandle;
pub(crate) use reporting::PlaybackReporter;
#[cfg(target_os = "windows")]
pub(crate) use windows_runtime::install_windows_app_handle;
#[cfg(target_os = "windows")]
pub(crate) use windows_runtime::spawn_controller_process_native_events;

#[cfg(target_os = "android")]
pub(crate) fn android_default_device_name(app: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager;

    app.try_state::<android_mobile_plugin::AndroidPlaybackRuntimeState>()
        .and_then(|state| state.bridge.default_device_name().ok())
        .filter(|name| !name.trim().is_empty())
}

pub(crate) fn create_playback_controller(
    _app: &tauri::AppHandle,
    app_handle: PlaybackEventAppHandle,
    persister: Box<dyn PlaybackStatePersister>,
    gapless_playback_enabled: bool,
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
        );
    }

    #[cfg(target_os = "android")]
    panic!("Android playback runtime state was not registered before controller setup.")
}
