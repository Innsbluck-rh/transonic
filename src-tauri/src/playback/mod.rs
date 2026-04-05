mod backend_shims;
mod controller;
mod native_events;
mod queue_sync;
mod reporting;

use crate::playback_state::PlaybackStatePersister;

#[cfg(target_os = "android")]
mod android_mobile_plugin;
#[cfg(target_os = "android")]
mod android_runtime;

use queue_sync::NoopQueueSyncGateway;
use reporting::TauriPlaybackReporter;

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

pub(crate) fn create_playback_controller(
    _app: &tauri::AppHandle,
    app_handle: PlaybackEventAppHandle,
    persister: Box<dyn PlaybackStatePersister>,
) -> PlaybackController {
    #[cfg(target_os = "android")]
    {
        use tauri::Manager;

        if let Some(runtime_state) =
            _app.try_state::<android_mobile_plugin::AndroidPlaybackRuntimeState>()
        {
            return PlaybackController::new(
                backend_shims::create_playback_backend(runtime_state.bridge.clone()),
                Box::new(TauriPlaybackReporter::new(app_handle)),
                Box::new(NoopQueueSyncGateway),
                Box::new(runtime_state.event_hub.clone()),
                persister,
            );
        }
    }

    #[cfg(target_os = "windows")]
    {
        let event_hub = backend_shims::SymphoniaPlaybackEventHub::default();
        return PlaybackController::new(
            backend_shims::create_playback_backend(event_hub.clone()),
            Box::new(TauriPlaybackReporter::new(app_handle)),
            Box::new(NoopQueueSyncGateway),
            Box::new(event_hub),
            persister,
        );
    }

    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        return PlaybackController::new(
            backend_shims::create_playback_backend(),
            Box::new(TauriPlaybackReporter::new(app_handle)),
            Box::new(NoopQueueSyncGateway),
            Box::new(NoopNativePlaybackEventSource),
            persister,
        );
    }

    #[cfg(target_os = "android")]
    panic!("Android playback runtime state was not registered before controller setup.")
}
