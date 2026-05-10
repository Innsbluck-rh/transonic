use std::sync::{
    mpsc::{self, Sender},
    Mutex, OnceLock,
};

use tauri::{AppHandle, Manager, Wry};

use crate::{
    commands::common::client, models::CapabilityMatrix, ActiveSessionState, CoverArtCacheState,
    PlaybackControllerState,
};

use super::PlaybackRuntimeContext;

static WINDOWS_APP_HANDLE: OnceLock<Mutex<Option<AppHandle<Wry>>>> = OnceLock::new();
static WINDOWS_EVENT_DISPATCHER: OnceLock<Sender<()>> = OnceLock::new();

pub fn install_windows_app_handle(app: AppHandle<Wry>) {
    let storage = WINDOWS_APP_HANDLE.get_or_init(|| Mutex::new(None));
    *storage.lock().unwrap() = Some(app);
}

fn cloned_app_handle() -> Option<AppHandle<Wry>> {
    WINDOWS_APP_HANDLE
        .get()
        .and_then(|storage| storage.lock().unwrap().clone())
}

#[derive(Clone)]
struct OwnedPlaybackRuntimeContext {
    client: opensubsonic_client::OpenSubsonicClient,
    capability_matrix: CapabilityMatrix,
    cover_art_cache: crate::cover_art_cache::CoverArtCache,
    profile_id: String,
}

impl OwnedPlaybackRuntimeContext {
    fn as_context(&self) -> PlaybackRuntimeContext<'_> {
        PlaybackRuntimeContext {
            client: &self.client,
            capability_matrix: &self.capability_matrix,
            cover_art_cache: Some(&self.cover_art_cache),
            profile_id: Some(&self.profile_id),
        }
    }
}

fn playback_runtime_context(app: &AppHandle<Wry>) -> Option<OwnedPlaybackRuntimeContext> {
    let sessions = app.try_state::<ActiveSessionState>()?;
    let (capability_matrix, profile_id) = {
        let guard = sessions.0.lock().unwrap();
        let session = guard.as_ref()?;
        (
            session.capability_matrix.clone(),
            session.profile_id.clone(),
        )
    };
    let cover_art_cache = app.try_state::<CoverArtCacheState>()?.0.clone();
    let client = client(app, &sessions.0).ok()?;

    Some(OwnedPlaybackRuntimeContext {
        client,
        capability_matrix,
        cover_art_cache,
        profile_id,
    })
}

pub fn spawn_controller_process_native_events() {
    if cloned_app_handle().is_none() {
        return;
    }

    if let Err(error) = controller_event_dispatcher().send(()) {
        log::error!(
            "windows_runtime: failed to schedule native playback event processing: {error}"
        );
    }
}

fn execute_controller_process_native_events(app: &AppHandle<Wry>) -> Result<(), String> {
    let playback = app.state::<PlaybackControllerState>();
    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let runtime_context = playback_runtime_context(app);
    let runtime_context_ref = runtime_context
        .as_ref()
        .map(OwnedPlaybackRuntimeContext::as_context);

    controller.process_native_events(runtime_context_ref.as_ref())?;
    Ok(())
}

fn controller_event_dispatcher() -> &'static Sender<()> {
    WINDOWS_EVENT_DISPATCHER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<()>();
        std::thread::Builder::new()
            .name("windows-playback-event-dispatcher".into())
            .spawn(move || {
                while rx.recv().is_ok() {
                    while rx.try_recv().is_ok() {}

                    let Some(app) = cloned_app_handle() else {
                        continue;
                    };
                    if let Err(error) = execute_controller_process_native_events(&app) {
                        log::error!(
                            "windows_runtime: failed to process native playback events: {error}"
                        );
                    }
                }
            })
            .expect("Failed to spawn windows playback event dispatcher");
        tx
    })
}
