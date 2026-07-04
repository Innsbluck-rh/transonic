use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use jni::{
    objects::{JObject, JString},
    sys::jlong,
    JNIEnv,
};
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event as _;

use crate::{
    commands::{common::client, playback::active_runtime_parts},
    models::{
        resolve_effective_stream_settings, CapabilityMatrix, MediaNotificationTap,
        NetworkCostState, PlaybackStatus,
    },
    ActiveSessionState, AppSettingsState, CoverArtCacheState, NetworkCostStateState,
    PlaybackControllerState,
};

use super::{
    android_mobile_plugin::AndroidPlaybackRuntimeState, native_events::PlaybackNativeEvent,
    PlaybackRuntimeContext,
};

static ANDROID_APP_HANDLE: OnceLock<Mutex<Option<AppHandle<Wry>>>> = OnceLock::new();
static PENDING_NOTIFICATION_TAP: AtomicBool = AtomicBool::new(false);
const ANDROID_NOTIFICATION_COVER_ART_SIZE: u32 = 512;

pub fn consume_pending_notification_tap() -> bool {
    PENDING_NOTIFICATION_TAP.swap(false, Ordering::SeqCst)
}

pub fn install_android_app_handle(app: AppHandle<Wry>) {
    let storage = ANDROID_APP_HANDLE.get_or_init(|| Mutex::new(None));
    *storage.lock().unwrap() = Some(app);
}

#[derive(Debug, Clone, Copy)]
enum AndroidControllerAction {
    Next,
    Prev,
    ProcessNativeEvents,
}

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

fn cloned_app_handle() -> Option<AppHandle<Wry>> {
    ANDROID_APP_HANDLE
        .get()
        .and_then(|storage| storage.lock().unwrap().clone())
}

fn enqueue_native_event(event: PlaybackNativeEvent) {
    let Some(app) = cloned_app_handle() else {
        return;
    };
    let Some(runtime_state) = app.try_state::<AndroidPlaybackRuntimeState>() else {
        return;
    };

    runtime_state.event_hub.push(event);
    spawn_controller_action(AndroidControllerAction::ProcessNativeEvents);
}

fn spawn_controller_action(action: AndroidControllerAction) {
    let Some(app) = cloned_app_handle() else {
        return;
    };

    std::thread::spawn(move || {
        if let Err(error) = execute_controller_action(&app, action) {
            log::error!("android_runtime: failed to execute native controller action: {error}");
        }
    });
}

pub(crate) fn spawn_android_artwork_update(app: AppHandle<Wry>) {
    std::thread::spawn(move || {
        if let Err(error) = update_current_artwork(&app) {
            log::warn!("android_runtime: failed to update media artwork: {error}");
        }
    });
}

fn update_current_artwork(app: &AppHandle<Wry>) -> Result<(), String> {
    let playback = app.state::<PlaybackControllerState>();
    let target = {
        let controller = playback
            .0
            .lock()
            .map_err(|_| "The playback controller state is unavailable.".to_string())?;
        controller.current_artwork_update_target()
    };
    let Some(target) = target else {
        return Ok(());
    };

    let Some(runtime_context) = playback_runtime_context(app) else {
        return Ok(());
    };
    let local_path = runtime_context.cover_art_cache.resolve_cover_art(
        &runtime_context.client,
        &runtime_context.profile_id,
        &target.cover_art_id,
        Some(ANDROID_NOTIFICATION_COVER_ART_SIZE),
    )?;

    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;
    let _ =
        controller.update_artwork_if_current(&target, local_path.to_string_lossy().to_string())?;
    Ok(())
}

fn execute_controller_action(
    app: &AppHandle<Wry>,
    action: AndroidControllerAction,
) -> Result<(), String> {
    {
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

        match action {
            AndroidControllerAction::Next => {
                let Some(runtime_context) = runtime_context_ref.as_ref() else {
                    return Err("No active playback session is available.".to_string());
                };
                controller.next(runtime_context)?;
            }
            AndroidControllerAction::Prev => {
                let Some(runtime_context) = runtime_context_ref.as_ref() else {
                    return Err("No active playback session is available.".to_string());
                };
                controller.prev(runtime_context)?;
            }
            AndroidControllerAction::ProcessNativeEvents => {}
        }
    }

    spawn_android_artwork_update(app.clone());
    Ok(())
}

fn decode_jstring(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Option<String> {
    env.get_string(value).ok().map(Into::into)
}

fn parse_network_cost_state(value: &str) -> Option<NetworkCostState> {
    match value {
        "unknown" => Some(NetworkCostState::Unknown),
        "metered" => Some(NetworkCostState::Metered),
        "unmetered" => Some(NetworkCostState::Unmetered),
        _ => None,
    }
}

pub(crate) fn apply_network_cost_state_str(
    app: &AppHandle<Wry>,
    raw_state: &str,
) -> Result<(), String> {
    let network_cost_state = parse_network_cost_state(raw_state)
        .ok_or_else(|| format!("Unknown network cost state: {raw_state}"))?;
    apply_network_cost_state(app, network_cost_state)
}

fn apply_network_cost_state(
    app: &AppHandle<Wry>,
    next_state: NetworkCostState,
) -> Result<(), String> {
    let network_cost_state = app
        .try_state::<NetworkCostStateState>()
        .ok_or_else(|| "The network cost state is unavailable.".to_string())?;
    let previous_state = {
        let mut guard = network_cost_state
            .0
            .lock()
            .map_err(|_| "The network cost state is unavailable.".to_string())?;
        let previous = *guard;
        if previous == next_state {
            return Ok(());
        }
        *guard = next_state;
        previous
    };

    let settings_state = app
        .try_state::<AppSettingsState>()
        .ok_or_else(|| "The app settings state is unavailable.".to_string())?;
    let playback_settings = {
        let guard = settings_state
            .0
            .lock()
            .map_err(|_| "The app settings state is unavailable.".to_string())?;
        guard.snapshot().0.playback
    };

    let previous_stream_settings =
        resolve_effective_stream_settings(&playback_settings, previous_state);
    let next_stream_settings = resolve_effective_stream_settings(&playback_settings, next_state);
    if previous_stream_settings == next_stream_settings {
        return Ok(());
    }

    let playback = app
        .try_state::<PlaybackControllerState>()
        .ok_or_else(|| "The playback controller state is unavailable.".to_string())?;
    let sessions = app
        .try_state::<ActiveSessionState>()
        .ok_or_else(|| "The active session state is unavailable.".to_string())?;
    let cover_art_cache = app
        .try_state::<CoverArtCacheState>()
        .ok_or_else(|| "The cover art cache state is unavailable.".to_string())?;
    let mut controller = playback
        .0
        .lock()
        .map_err(|_| "The playback controller state is unavailable.".to_string())?;

    let update_result =
        if let Ok((client, capability_matrix, profile_id)) = active_runtime_parts(app, &sessions) {
            let runtime_context = PlaybackRuntimeContext {
                client: &client,
                capability_matrix: &capability_matrix,
                cover_art_cache: Some(&cover_art_cache.0),
                profile_id: Some(&profile_id),
            };
            controller.set_stream_settings(
                next_stream_settings.stream_mode,
                next_stream_settings.transcoding_bitrate_limit,
                next_stream_settings.transcoding_codec,
                Some(&runtime_context),
            )
        } else {
            controller.set_stream_settings(
                next_stream_settings.stream_mode,
                next_stream_settings.transcoding_bitrate_limit,
                next_stream_settings.transcoding_codec,
                None,
            )
        };

    if let Err(error) = update_result {
        log::warn!(
            "android_runtime: failed to refresh stream settings after network change: {error}"
        );
    }
    Ok(())
}

fn spawn_network_cost_state_apply(app: AppHandle<Wry>, next_state: NetworkCostState) {
    std::thread::spawn(move || {
        if let Err(error) = apply_network_cost_state(&app, next_state) {
            log::warn!("android_runtime: failed to apply network cost state: {error}");
        }
    });
}

fn sync_playback_status_for_android_resume(app: &AppHandle<Wry>) -> Option<PlaybackStatus> {
    let playback = app.try_state::<PlaybackControllerState>()?;
    let runtime_context = playback_runtime_context(app);
    let runtime_context_ref = runtime_context
        .as_ref()
        .map(OwnedPlaybackRuntimeContext::as_context);
    let mut controller = match playback.0.lock() {
        Ok(controller) => controller,
        Err(_) => {
            log::warn!("android_runtime: playback controller state unavailable during resume");
            return None;
        }
    };

    match controller.synced_state_with_context(runtime_context_ref.as_ref()) {
        Ok(status) => Some(status),
        Err(error) => {
            log::warn!("android_runtime: failed to sync playback state during resume: {error}");
            None
        }
    }
}

fn spawn_android_resume_sync(app: AppHandle<Wry>) {
    std::thread::spawn(move || {
        let playback_status = sync_playback_status_for_android_resume(&app);
        crate::connect::sync_after_android_resume(&app, playback_status);
    });
}

#[no_mangle]
pub extern "system" fn Java_com_innsb_transonic_playback_RustPlaybackBridge_enqueuePlaybackEvent(
    mut env: JNIEnv<'_>,
    _this: JObject<'_>,
    kind: JString<'_>,
    position_ms: jlong,
    error: JObject<'_>,
) {
    let Some(kind) = decode_jstring(&mut env, &kind) else {
        return;
    };
    let message = if error.is_null() {
        None
    } else {
        decode_jstring(&mut env, &JString::from(error))
    };
    let position_ms = if position_ms.is_negative() {
        0
    } else {
        u32::try_from(position_ms as u64).unwrap_or(u32::MAX)
    };

    let event = match kind.as_str() {
        "buffering" => PlaybackNativeEvent::Buffering { position_ms },
        "ready" => PlaybackNativeEvent::Ready { position_ms },
        "playing" => PlaybackNativeEvent::Playing { position_ms },
        "paused" => PlaybackNativeEvent::Paused { position_ms },
        "seek_processed" => PlaybackNativeEvent::SeekProcessed { position_ms },
        "gapless_transition" => PlaybackNativeEvent::GaplessTransition,
        "error" => PlaybackNativeEvent::Error {
            message: message.unwrap_or_else(|| "Android playback failed.".to_string()),
            handled: false,
        },
        "handled_error" => PlaybackNativeEvent::Error {
            message: message.unwrap_or_else(|| "Android playback failed.".to_string()),
            handled: true,
        },
        "ended" => PlaybackNativeEvent::Ended,
        _ => return,
    };

    enqueue_native_event(event);
}

#[no_mangle]
pub extern "system" fn Java_com_innsb_transonic_playback_RustPlaybackBridge_dispatchControllerAction(
    mut env: JNIEnv<'_>,
    _this: JObject<'_>,
    action: JString<'_>,
) {
    let Some(action) = decode_jstring(&mut env, &action) else {
        return;
    };

    let action = match action.as_str() {
        "next" => AndroidControllerAction::Next,
        "prev" => AndroidControllerAction::Prev,
        _ => return,
    };

    spawn_controller_action(action);
}

#[no_mangle]
pub extern "system" fn Java_com_innsb_transonic_playback_RustPlaybackBridge_notifyAppResumedFromNotification(
    _env: JNIEnv<'_>,
    _this: JObject<'_>,
) {
    let Some(app) = cloned_app_handle() else {
        log::info!("android_runtime: app handle not available, storing pending notification tap");
        PENDING_NOTIFICATION_TAP.store(true, Ordering::SeqCst);
        return;
    };

    if let Err(error) = (MediaNotificationTap {}).emit(&app) {
        log::error!("android_runtime: failed to emit MediaNotificationTap event: {error}");
        PENDING_NOTIFICATION_TAP.store(true, Ordering::SeqCst);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_innsb_transonic_playback_RustPlaybackBridge_notifyAppResumed(
    _env: JNIEnv<'_>,
    _this: JObject<'_>,
) {
    let Some(app) = cloned_app_handle() else {
        return;
    };

    spawn_android_resume_sync(app);
}

#[no_mangle]
pub extern "system" fn Java_com_innsb_transonic_playback_RustPlaybackBridge_updateNetworkCostState(
    mut env: JNIEnv<'_>,
    _this: JObject<'_>,
    state: JString<'_>,
) {
    let Some(raw_state) = decode_jstring(&mut env, &state) else {
        return;
    };
    let Some(network_cost_state) = parse_network_cost_state(raw_state.as_str()) else {
        log::warn!("android_runtime: ignored unknown network cost state: {raw_state}");
        return;
    };
    let Some(app) = cloned_app_handle() else {
        log::info!("android_runtime: app handle not available, ignoring network cost update");
        return;
    };

    spawn_network_cost_state_apply(app, network_cost_state);
}
