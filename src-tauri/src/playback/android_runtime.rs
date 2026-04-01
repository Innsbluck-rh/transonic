use std::sync::{Mutex, OnceLock};

use jni::{
    objects::{JObject, JString},
    sys::jlong,
    JNIEnv,
};
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event;

use crate::{
    commands::common::client,
    models::{CapabilityMatrix, PlaybackNativeDirty},
    ActiveSessionState, PlaybackControllerState,
};

use super::{
    android_mobile_plugin::AndroidPlaybackRuntimeState, native_events::PlaybackNativeEvent,
    PlaybackRuntimeContext,
};

static ANDROID_APP_HANDLE: OnceLock<Mutex<Option<AppHandle<Wry>>>> = OnceLock::new();

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
}

impl OwnedPlaybackRuntimeContext {
    fn as_context(&self) -> PlaybackRuntimeContext<'_> {
        PlaybackRuntimeContext {
            client: &self.client,
            capability_matrix: &self.capability_matrix,
        }
    }
}

fn playback_runtime_context(app: &AppHandle<Wry>) -> Option<OwnedPlaybackRuntimeContext> {
    let sessions = app.try_state::<ActiveSessionState>()?;
    let capability_matrix = {
        let guard = sessions.0.lock().unwrap();
        guard.as_ref()?.capability_matrix.clone()
    };
    let client = client(app, &sessions.0).ok()?;

    Some(OwnedPlaybackRuntimeContext {
        client,
        capability_matrix,
    })
}

fn cloned_app_handle() -> Option<AppHandle<Wry>> {
    ANDROID_APP_HANDLE
        .get()
        .and_then(|storage| storage.lock().unwrap().clone())
}

fn emit_native_dirty(app: &AppHandle<Wry>) {
    let _ = PlaybackNativeDirty.emit(app);
}

fn enqueue_native_event(event: PlaybackNativeEvent) {
    let Some(app) = cloned_app_handle() else {
        return;
    };
    let Some(runtime_state) = app.try_state::<AndroidPlaybackRuntimeState>() else {
        return;
    };

    let should_process_now = matches!(event, PlaybackNativeEvent::Ended);
    runtime_state.event_hub.push(event);
    emit_native_dirty(&app);

    if should_process_now {
        spawn_controller_action(AndroidControllerAction::ProcessNativeEvents);
    }
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

fn execute_controller_action(
    app: &AppHandle<Wry>,
    action: AndroidControllerAction,
) -> Result<(), String> {
    let playback = app.state::<PlaybackControllerState>();
    let mut controller = playback.0.lock().unwrap();
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

    Ok(())
}

fn decode_jstring(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Option<String> {
    env.get_string(value).ok().map(Into::into)
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
        "error" => PlaybackNativeEvent::Error {
            message: message.unwrap_or_else(|| "Android playback failed.".to_string()),
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
