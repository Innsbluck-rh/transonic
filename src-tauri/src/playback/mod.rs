mod backend_shims;
mod controller;
mod native_events;
mod queue_sync;
mod reporting;

#[cfg(target_os = "android")]
mod android_mobile_plugin;
#[cfg(target_os = "android")]
mod android_runtime;

use queue_sync::NoopQueueSyncGateway;
use reporting::TauriPlaybackReporter;

#[cfg(not(target_os = "android"))]
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
            );
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        return PlaybackController::new(
            backend_shims::create_playback_backend(),
            Box::new(TauriPlaybackReporter::new(app_handle)),
            Box::new(NoopQueueSyncGateway),
            Box::new(NoopNativePlaybackEventSource),
        );
    }

    #[cfg(target_os = "android")]
    panic!("Android playback runtime state was not registered before controller setup.")
}
