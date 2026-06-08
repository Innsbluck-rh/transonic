use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri_specta::Event;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        http::{header::AUTHORIZATION, HeaderValue},
        Message,
    },
};

use crate::{
    commands::{common::service, playback::active_runtime_parts},
    models::{
        AuthInput, ConnectDevicePresence, ConnectDevicesUpdated, ConnectPlaybackState,
        ConnectRuntimeStatus, ConnectSettings, ConnectSharedPlaybackState,
        ConnectSharedPlaybackUpdated, PlaybackStatus,
    },
    playback::PlaybackRuntimeContext,
    ActiveSessionState, AppSettingsState, CoverArtCacheState, PlaybackControllerState,
};

const CONNECT_PROTOCOL_VERSION: u32 = 2;
const TYPE_PLAYBACK_SHARED_SNAPSHOT: &str = "playback.shared.snapshot";
const TYPE_PLAYBACK_SHARED_UPDATED: &str = "playback.shared.updated";
const UPDATE_REASON_SNAPSHOT: &str = "snapshot";
const UPDATE_REASON_ACTIVE_OFFLINE: &str = "activeOffline";

#[derive(Clone)]
pub(crate) struct ConnectState(pub Arc<ConnectRuntime>);

impl Default for ConnectState {
    fn default() -> Self {
        Self(Arc::new(ConnectRuntime::default()))
    }
}

#[derive(Default)]
pub(crate) struct ConnectRuntime {
    inner: Mutex<ConnectRuntimeInner>,
}

#[derive(Default)]
struct ConnectRuntimeInner {
    generation: u64,
    status: ConnectRuntimeStatus,
    sender: Option<mpsc::UnboundedSender<ConnectOutbound>>,
    device_id: Option<String>,
    devices: Vec<ConnectDevicePresence>,
    shared_playback: Option<ConnectSharedPlaybackState>,
    applying_shared_seq: Option<u32>,
}

enum ConnectOutbound {
    PlaybackReport {
        base_seq: u32,
        state: ConnectPlaybackState,
    },
    Envelope(ConnectOutboundEnvelope),
}

struct ConnectOutboundEnvelope {
    id: Option<String>,
    message_type: &'static str,
    payload: serde_json::Value,
}

struct SharedPlaybackUpdate {
    shared: ConnectSharedPlaybackState,
    should_apply: bool,
    allow_preserve_active_track: bool,
}

fn should_apply_shared_playback_to_backend(
    message_type: &str,
    shared: &ConnectSharedPlaybackState,
    runtime_should_apply: bool,
    is_active_device: bool,
    should_preserve_active_snapshot: bool,
) -> bool {
    if !runtime_should_apply {
        return false;
    }

    if shared.update_reason.as_deref() == Some(UPDATE_REASON_ACTIVE_OFFLINE) {
        return false;
    }

    let is_snapshot = message_type == TYPE_PLAYBACK_SHARED_SNAPSHOT
        || shared.update_reason.as_deref() == Some(UPDATE_REASON_SNAPSHOT);
    if is_snapshot && is_active_device && should_preserve_active_snapshot {
        return false;
    }

    true
}

pub(crate) fn should_defer_local_playback_restore_for_connect(
    settings: &ConnectSettings,
    subsonic_server_url: &str,
) -> bool {
    if !settings.enabled || settings.device_id.trim().is_empty() {
        return false;
    }

    resolve_connect_server_url(settings, Some(subsonic_server_url))
        .ok()
        .flatten()
        .is_some()
}

impl Default for ConnectRuntimeStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            connected: false,
            message: None,
            server_url: None,
            device_id: None,
            seq: 0,
        }
    }
}

impl ConnectRuntime {
    fn begin_restart(&self) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        inner.generation = inner.generation.wrapping_add(1);
        inner.sender = None;
        inner.device_id = None;
        inner.devices.clear();
        inner.shared_playback = None;
        inner.status.connected = false;
        inner.status.message = Some("connect: restarting".to_string());
        inner.status.device_id = None;
        inner.generation
    }

    fn is_current(&self, generation: u64) -> bool {
        self.inner.lock().unwrap().generation == generation
    }

    fn set_disabled(&self, generation: u64) {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return;
        }
        inner.sender = None;
        inner.device_id = None;
        inner.devices.clear();
        inner.shared_playback = None;
        let seq = inner.status.seq;
        inner.status = ConnectRuntimeStatus::default();
        inner.status.seq = seq;
    }

    fn set_disconnected(&self, generation: u64, server_url: Option<String>, message: String) {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return;
        }
        inner.sender = None;
        inner.device_id = None;
        inner.devices.clear();
        inner.shared_playback = None;
        inner.status.enabled = server_url.is_some();
        inner.status.connected = false;
        inner.status.server_url = server_url;
        inner.status.message = Some(message);
    }

    fn set_connected(
        &self,
        generation: u64,
        server_url: String,
        device_id: String,
        sender: mpsc::UnboundedSender<ConnectOutbound>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return;
        }
        inner.sender = Some(sender);
        inner.device_id = Some(device_id.clone());
        inner.status.enabled = true;
        inner.status.connected = true;
        inner.status.server_url = Some(server_url);
        inner.status.device_id = Some(device_id);
        inner.status.message = Some("connect: websocket connected".to_string());
    }

    fn set_devices(
        &self,
        generation: u64,
        devices: Vec<ConnectDevicePresence>,
    ) -> Option<Vec<ConnectDevicePresence>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return None;
        }
        inner.devices = devices;
        Some(inner.devices.clone())
    }

    fn set_shared_playback(
        &self,
        generation: u64,
        shared: ConnectSharedPlaybackState,
    ) -> Option<SharedPlaybackUpdate> {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return None;
        }
        if inner
            .shared_playback
            .as_ref()
            .is_some_and(|current| shared.seq < current.seq)
        {
            return None;
        }
        inner.status.seq = shared.seq;
        let device_id = inner.device_id.clone();
        let was_active_device = device_id.as_ref().is_some_and(|device_id| {
            inner
                .shared_playback
                .as_ref()
                .and_then(|current| current.active_device_id.as_deref())
                == Some(device_id.as_str())
        });
        let is_active_device = device_id.as_ref().is_some_and(|device_id| {
            shared.active_device_id.as_deref() == Some(device_id.as_str())
        });
        let should_skip_own_report = device_id.as_ref().is_some_and(|device_id| {
            shared.active_device_id.as_deref() == Some(device_id.as_str())
                && shared.updated_by_device_id.as_deref() == Some(device_id.as_str())
                && shared.update_reason.as_deref() == Some("report")
        });
        inner.shared_playback = Some(shared.clone());
        Some(SharedPlaybackUpdate {
            shared,
            should_apply: !should_skip_own_report,
            allow_preserve_active_track: was_active_device && is_active_device,
        })
    }

    fn begin_apply_shared(&self, generation: u64, seq: u32) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return false;
        }
        inner.applying_shared_seq = Some(seq);
        true
    }

    fn end_apply_shared(&self, seq: u32) {
        let mut inner = self.inner.lock().unwrap();
        if inner.applying_shared_seq == Some(seq) {
            inner.applying_shared_seq = None;
        }
    }

    pub(crate) fn publish_status(&self, status: PlaybackStatus) {
        let outbound = {
            let inner = self.inner.lock().unwrap();
            if !inner.status.connected || inner.applying_shared_seq.is_some() {
                return;
            }
            let Some(shared) = inner.shared_playback.as_ref() else {
                return;
            };
            let Some(device_id) = inner.device_id.as_deref() else {
                return;
            };
            if shared.active_device_id.as_deref() != Some(device_id) {
                return;
            }
            inner.sender.clone().map(|sender| {
                (
                    sender,
                    ConnectOutbound::PlaybackReport {
                        base_seq: shared.seq,
                        state: ConnectPlaybackState::from(status),
                    },
                )
            })
        };
        if let Some((sender, outbound)) = outbound {
            let _ = sender.send(outbound);
        }
    }

    pub(crate) fn request_transfer_playback(&self, target_device_id: String) -> Result<(), String> {
        let target_device_id = target_device_id.trim().to_string();
        {
            let inner = self.inner.lock().unwrap();
            Self::validate_transfer_playback_target(&inner, &target_device_id)?;
        }

        if !self.send_playback_command(
            "transferPlayback",
            serde_json::json!({ "targetDeviceId": target_device_id }),
        )? {
            return Err("connect: websocket is not connected".to_string());
        }
        Ok(())
    }

    pub(crate) fn send_playback_command(
        &self,
        op: &'static str,
        payload: serde_json::Value,
    ) -> Result<bool, String> {
        let (sender, base_seq) = {
            let inner = self.inner.lock().unwrap();
            if !inner.status.connected {
                return Ok(false);
            }
            let sender = inner
                .sender
                .clone()
                .ok_or_else(|| "connect: websocket is not connected".to_string())?;
            let base_seq = inner
                .shared_playback
                .as_ref()
                .map(|shared| shared.seq)
                .unwrap_or(0);
            (sender, base_seq)
        };

        let mut command = match payload {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        command.insert(
            "commandId".to_string(),
            serde_json::json!(connect_message_id(op)),
        );
        command.insert("baseSeq".to_string(), serde_json::json!(base_seq));
        command.insert("op".to_string(), serde_json::json!(op));
        sender
            .send(ConnectOutbound::Envelope(ConnectOutboundEnvelope {
                id: command
                    .get("commandId")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned),
                message_type: "playback.command.request",
                payload: serde_json::Value::Object(command),
            }))
            .map_err(|_| "connect: websocket writer is not available".to_string())?;
        Ok(true)
    }

    pub(crate) fn status(&self) -> ConnectRuntimeStatus {
        self.inner.lock().unwrap().status.clone()
    }

    pub(crate) fn devices(&self) -> Vec<ConnectDevicePresence> {
        self.inner.lock().unwrap().devices.clone()
    }

    pub(crate) fn shared_playback(&self) -> Option<ConnectSharedPlaybackState> {
        self.inner.lock().unwrap().shared_playback.clone()
    }

    fn device_id(&self) -> Option<String> {
        self.inner.lock().unwrap().device_id.clone()
    }

    fn validate_transfer_playback_target(
        inner: &ConnectRuntimeInner,
        target_device_id: &str,
    ) -> Result<(), String> {
        if target_device_id.is_empty() {
            return Err("connect: transfer target device is unavailable".to_string());
        }
        if !inner.status.connected {
            return Err("connect: websocket is not connected".to_string());
        }
        if !inner
            .devices
            .iter()
            .any(|device| device.device_id == target_device_id && device.online)
        {
            return Err("connect: target device is offline".to_string());
        }
        let shared = inner
            .shared_playback
            .as_ref()
            .ok_or_else(|| "connect: shared playback state is unavailable".to_string())?;
        if shared.active_device_id.as_deref() == Some(target_device_id) {
            return Err("connect: target device is already active".to_string());
        }
        if shared.state.queue.is_empty() {
            return Err("connect: playback queue is empty".to_string());
        }
        Ok(())
    }
}

pub(crate) struct ConnectPlaybackReporter {
    runtime: Arc<ConnectRuntime>,
}

impl ConnectPlaybackReporter {
    pub(crate) fn from_app(app: &tauri::AppHandle) -> Self {
        Self {
            runtime: app.state::<ConnectState>().0.clone(),
        }
    }
}

impl crate::playback::PlaybackReporter for ConnectPlaybackReporter {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
        self.runtime.publish_status(status.clone());
        Ok(())
    }
}

pub(crate) fn restart(app: &tauri::AppHandle) {
    let runtime = app.state::<ConnectState>().0.clone();
    let generation = runtime.begin_restart();
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        run_connect_loop(app, runtime, generation).await;
    });
}

async fn run_connect_loop(app: tauri::AppHandle, runtime: Arc<ConnectRuntime>, generation: u64) {
    loop {
        if !runtime.is_current(generation) {
            return;
        }

        match run_connect_once(&app, runtime.clone(), generation).await {
            Ok(ConnectRunResult::Disabled) => {
                runtime.set_disabled(generation);
                return;
            }
            Ok(ConnectRunResult::Restart) => {}
            Err(error) => {
                let server_url = current_connect_server_url(&app);
                log::warn!("connect: {error}");
                runtime.set_disconnected(generation, server_url, error);
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

enum ConnectRunResult {
    Disabled,
    Restart,
}

async fn run_connect_once(
    app: &tauri::AppHandle,
    runtime: Arc<ConnectRuntime>,
    generation: u64,
) -> Result<ConnectRunResult, String> {
    let settings = {
        let settings_state = app.state::<AppSettingsState>();
        let guard = settings_state
            .0
            .lock()
            .map_err(|_| "connect: settings state unavailable".to_string())?;
        guard.snapshot().0
    };

    if !settings.connect.enabled {
        return Ok(ConnectRunResult::Disabled);
    }

    let (active_session, auth_input) = {
        let sessions = app.state::<ActiveSessionState>();
        service(app)?.active_auth_input(&sessions.0)?
    };

    let Some(connect_server_url) = resolve_connect_server_url(
        &settings.connect,
        Some(&active_session.normalized_server_url),
    )?
    else {
        return Ok(ConnectRunResult::Disabled);
    };

    let device_id = settings.connect.device_id.trim().to_string();
    if device_id.is_empty() {
        return Err("connect: deviceId is missing".to_string());
    }

    let device_name = settings
        .connect
        .device_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| device_id.clone());

    let login_response = login_connect_server(
        &connect_server_url,
        ConnectLoginRequest {
            upstream_server_url: active_session.normalized_server_url.clone(),
            auth: auth_input,
            device: ConnectDeviceRequest {
                device_id: device_id.clone(),
                display_name: device_name,
                platform: std::env::consts::OS.to_string(),
                app_version: app.package_info().version.to_string(),
            },
        },
    )
    .await?;

    let ws_url = websocket_url(&connect_server_url)?;
    let mut request = ws_url
        .as_str()
        .into_client_request()
        .map_err(|error| format!("connect: failed to build websocket request: {error}"))?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", login_response.access_token))
            .map_err(|error| format!("connect: invalid authorization header: {error}"))?,
    );

    let (ws, _) = connect_async(request)
        .await
        .map_err(|error| format!("connect: websocket failed: {error}"))?;
    let (mut write, mut read) = ws.split();
    let (sender, mut receiver) = mpsc::unbounded_channel::<ConnectOutbound>();
    runtime.set_connected(
        generation,
        connect_server_url.clone(),
        login_response.device_id.clone(),
        sender,
    );

    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));

    loop {
        tokio::select! {
            outbound = receiver.recv() => {
                match outbound {
                    Some(ConnectOutbound::PlaybackReport { base_seq, state }) => {
                        let envelope = ConnectEnvelope {
                            id: Some(connect_message_id("playback-report")),
                            message_type: "playback.report",
                            protocol_version: CONNECT_PROTOCOL_VERSION,
                            device_id: &login_response.device_id,
                            payload: PlaybackReportPayload { base_seq, state },
                        };
                        write_json_message(&mut write, &envelope).await?;
                    }
                    Some(ConnectOutbound::Envelope(outbound)) => {
                        let envelope = ConnectEnvelope {
                            id: outbound.id,
                            message_type: outbound.message_type,
                            protocol_version: CONNECT_PROTOCOL_VERSION,
                            device_id: &login_response.device_id,
                            payload: outbound.payload,
                        };
                        write_json_message(&mut write, &envelope).await?;
                    }
                    None => return Ok(ConnectRunResult::Restart),
                }
            }
            _ = heartbeat.tick() => {
                let envelope = ConnectEnvelope {
                    id: None,
                    message_type: "presence.heartbeat",
                    protocol_version: CONNECT_PROTOCOL_VERSION,
                    device_id: &login_response.device_id,
                    payload: serde_json::json!({}),
                };
                write_json_message(&mut write, &envelope).await?;
            }
            incoming = read.next() => {
                match incoming {
                    Some(Ok(message)) => {
                        handle_incoming_message(app, runtime.clone(), generation, message).await?;
                    }
                    Some(Err(error)) => return Err(format!("connect: websocket read failed: {error}")),
                    None => return Ok(ConnectRunResult::Restart),
                }
            }
        }
    }
}

async fn write_json_message<S, T>(write: &mut S, value: &T) -> Result<(), String>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
    T: Serialize,
{
    let json = serde_json::to_string(value)
        .map_err(|error| format!("connect: failed to serialize websocket message: {error}"))?;
    write
        .send(Message::Text(json.into()))
        .await
        .map_err(|error| format!("connect: websocket write failed: {error}"))
}

async fn handle_incoming_message(
    app: &tauri::AppHandle,
    runtime: Arc<ConnectRuntime>,
    generation: u64,
    message: Message,
) -> Result<(), String> {
    let text = if message.is_text() {
        message
            .into_text()
            .map_err(|error| format!("connect: websocket text decode failed: {error}"))?
            .to_string()
    } else if message.is_binary() {
        String::from_utf8(message.into_data().to_vec())
            .map_err(|error| format!("connect: websocket binary decode failed: {error}"))?
    } else {
        return Ok(());
    };

    let envelope: IncomingEnvelope = serde_json::from_str(&text)
        .map_err(|error| format!("connect: websocket message decode failed: {error}"))?;
    let Some(payload) = envelope.payload else {
        return Ok(());
    };

    match envelope.message_type.as_str() {
        "presence.updated" => {
            let payload: PresenceUpdatedPayload = serde_json::from_value(payload)
                .map_err(|error| format!("connect: presence update decode failed: {error}"))?;
            if let Some(devices) = runtime.set_devices(generation, payload.devices) {
                emit_devices_updated(app, devices);
            }
        }
        TYPE_PLAYBACK_SHARED_SNAPSHOT | TYPE_PLAYBACK_SHARED_UPDATED => {
            let message_type = envelope.message_type.as_str();
            let shared: ConnectSharedPlaybackState = serde_json::from_value(payload)
                .map_err(|error| format!("connect: shared playback decode failed: {error}"))?;
            if let Some(update) = runtime.set_shared_playback(generation, shared) {
                emit_shared_playback_updated(app, update.shared.clone());
                let is_active_device = runtime.device_id().as_deref().is_some_and(|device_id| {
                    update.shared.active_device_id.as_deref() == Some(device_id)
                });
                let should_preserve_active_snapshot =
                    is_active_device && should_preserve_active_connect_snapshot(app);
                if should_apply_shared_playback_to_backend(
                    message_type,
                    &update.shared,
                    update.should_apply,
                    is_active_device,
                    should_preserve_active_snapshot,
                ) {
                    if let Err(error) = apply_shared_playback_state(
                        app.clone(),
                        runtime.clone(),
                        generation,
                        update.shared,
                        update.allow_preserve_active_track,
                    )
                    .await
                    {
                        log::warn!("connect: shared playback apply failed: {error}");
                    }
                }
            }
        }
        "playback.apply" => {
            let payload: PlaybackApplyPayload = serde_json::from_value(payload)
                .map_err(|error| format!("connect: playback apply decode failed: {error}"))?;
            if let Err(error) = execute_playback_apply(app.clone(), payload).await {
                log::warn!("connect: playback apply failed: {error}");
            }
        }
        "playback.command.error" => {
            let payload: PlaybackCommandErrorPayload = serde_json::from_value(payload)
                .map_err(|error| format!("connect: command error decode failed: {error}"))?;
            log::warn!(
                "connect: command failed command_id={} message={}",
                payload.command_id.unwrap_or_default(),
                payload.message
            );
        }
        _ => {}
    }

    Ok(())
}

async fn apply_shared_playback_state(
    app: tauri::AppHandle,
    runtime: Arc<ConnectRuntime>,
    generation: u64,
    shared: ConnectSharedPlaybackState,
    allow_preserve_active_track: bool,
) -> Result<(), String> {
    let device_id = runtime
        .device_id()
        .ok_or_else(|| "connect: device id is unavailable".to_string())?;
    let is_active = shared.active_device_id.as_deref() == Some(device_id.as_str());
    if !runtime.begin_apply_shared(generation, shared.seq) {
        return Ok(());
    }
    let seq = shared.seq;
    let result = tauri::async_runtime::spawn_blocking(move || {
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

        if let Some((active_client, active_capability_matrix, active_profile_id, cover_art_cache)) =
            runtime_context.as_ref()
        {
            let context = PlaybackRuntimeContext {
                client: active_client,
                capability_matrix: active_capability_matrix,
                cover_art_cache: Some(cover_art_cache),
                profile_id: Some(active_profile_id),
            };
            controller.apply_connect_shared_state(
                Some(&context),
                shared.state,
                is_active,
                allow_preserve_active_track,
            )
        } else {
            controller.apply_connect_shared_state(
                None,
                shared.state,
                is_active,
                allow_preserve_active_track,
            )
        }
    })
    .await
    .map_err(|error| format!("connect: apply shared playback task failed: {error}"));
    runtime.end_apply_shared(seq);
    let result = result?;
    result.map(|_| ())
}

fn should_preserve_active_connect_snapshot(app: &tauri::AppHandle) -> bool {
    let Some(active_profile_id) = app
        .state::<ActiveSessionState>()
        .0
        .lock()
        .ok()
        .and_then(|session| session.as_ref().map(|session| session.profile_id.clone()))
    else {
        return false;
    };

    app.state::<PlaybackControllerState>()
        .0
        .lock()
        .ok()
        .is_some_and(|controller| {
            controller.can_preserve_active_connect_snapshot_for_profile(&active_profile_id)
        })
}

async fn execute_playback_apply(
    app: tauri::AppHandle,
    payload: PlaybackApplyPayload,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
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

        let mut controller = playback
            .0
            .lock()
            .map_err(|_| "The playback controller state is unavailable.".to_string())?;

        match payload.op.as_str() {
            "play" => controller.play(&runtime_context).map(|_| ()),
            "pause" => controller
                .pause_with_context(Some(&runtime_context))
                .map(|_| ()),
            "stop" => controller
                .stop_with_context(Some(&runtime_context))
                .map(|_| ()),
            "seek" => {
                let position_ms = payload
                    .args
                    .as_ref()
                    .and_then(|args| args.position_ms)
                    .ok_or_else(|| "connect: seek requires positionMs".to_string())?;
                controller.seek(&runtime_context, position_ms).map(|_| ())
            }
            "next" => controller.next(&runtime_context).map(|_| ()),
            "prev" => controller.prev(&runtime_context).map(|_| ()),
            _ => Err(format!(
                "connect: unsupported playback apply op {:?}",
                payload.op
            )),
        }
    })
    .await
    .map_err(|error| format!("connect: playback apply task failed: {error}"))?
}

fn emit_devices_updated(app: &tauri::AppHandle, devices: Vec<ConnectDevicePresence>) {
    if let Err(error) = (ConnectDevicesUpdated { devices }).emit(app) {
        log::warn!("connect: failed to emit devices update: {error}");
    }
}

fn emit_shared_playback_updated(
    app: &tauri::AppHandle,
    shared_playback: ConnectSharedPlaybackState,
) {
    if let Err(error) = (ConnectSharedPlaybackUpdated { shared_playback }).emit(app) {
        log::warn!("connect: failed to emit shared playback update: {error}");
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectLoginRequest {
    #[serde(rename = "upstreamServerUrl")]
    upstream_server_url: String,
    auth: AuthInput,
    device: ConnectDeviceRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectDeviceRequest {
    device_id: String,
    display_name: String,
    platform: String,
    app_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectLoginResponse {
    access_token: String,
    device_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectEnvelope<'a, T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "type")]
    message_type: &'static str,
    protocol_version: u32,
    device_id: &'a str,
    payload: T,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackReportPayload {
    base_seq: u32,
    state: ConnectPlaybackState,
}

#[derive(Debug, Deserialize)]
struct IncomingEnvelope {
    #[serde(rename = "type")]
    message_type: String,
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PresenceUpdatedPayload {
    devices: Vec<ConnectDevicePresence>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackApplyPayload {
    #[allow(dead_code)]
    command_id: Option<String>,
    op: String,
    args: Option<PlaybackApplyArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackApplyArgs {
    position_ms: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackCommandErrorPayload {
    command_id: Option<String>,
    message: String,
}

async fn login_connect_server(
    connect_server_url: &str,
    request: ConnectLoginRequest,
) -> Result<ConnectLoginResponse, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{connect_server_url}/v1/auth/login"))
        .json(&request)
        .send()
        .await
        .map_err(|error| format!("connect: login request failed: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("connect: login failed with HTTP {status}: {body}"));
    }

    response
        .json::<ConnectLoginResponse>()
        .await
        .map_err(|error| format!("connect: login response decode failed: {error}"))
}

fn normalize_connect_server_url(
    raw_url: Option<&str>,
    allow_insecure: bool,
    connect_server_port: Option<u16>,
) -> Result<Option<String>, String> {
    let Some(raw_url) = raw_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let mut parsed =
        Url::parse(raw_url).map_err(|_| "connect: Connect Server URL is invalid".to_string())?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("connect: Connect Server URL must not include credentials".to_string());
    }

    match parsed.scheme() {
        "http" => {
            let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
            let loopback = matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1");
            if !loopback && !allow_insecure {
                return Err("connect: LAN HTTP requires allowInsecureConnectServer".to_string());
            }
        }
        "https" => {}
        _ => {
            return Err(
                "connect: Connect Server URL must start with http:// or https://".to_string(),
            )
        }
    }

    let path = parsed.path().trim_end_matches('/').to_string();
    parsed.set_path(&path);
    if let Some(port) = connect_server_port {
        parsed
            .set_port(Some(port))
            .map_err(|_| "connect: failed to set Connect Server port".to_string())?;
    }
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(Some(parsed.to_string().trim_end_matches('/').to_string()))
}

fn resolve_connect_server_url(
    settings: &ConnectSettings,
    subsonic_server_url: Option<&str>,
) -> Result<Option<String>, String> {
    if !settings.enabled {
        return Ok(None);
    }

    if settings.use_subsonic_server_host {
        let Some(subsonic_server_url) = subsonic_server_url else {
            return Err("connect: active Subsonic server URL is unavailable".to_string());
        };
        return derive_connect_server_url_from_subsonic(
            subsonic_server_url,
            settings.connect_server_port,
            settings.allow_insecure_connect_server,
        );
    }

    normalize_connect_server_url(
        settings.connect_server_host.as_deref(),
        settings.allow_insecure_connect_server,
        Some(settings.connect_server_port),
    )
}

fn derive_connect_server_url_from_subsonic(
    subsonic_server_url: &str,
    connect_server_port: u16,
    allow_insecure: bool,
) -> Result<Option<String>, String> {
    let mut parsed = Url::parse(subsonic_server_url)
        .map_err(|_| "connect: Subsonic Server URL is invalid".to_string())?;
    parsed.set_path("");
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed
        .set_port(Some(connect_server_port))
        .map_err(|_| "connect: failed to set Connect Server port".to_string())?;

    let allow_derived_http = allow_insecure || parsed.scheme() == "http";
    normalize_connect_server_url(Some(parsed.as_str()), allow_derived_http, None)
}

fn websocket_url(connect_server_url: &str) -> Result<Url, String> {
    let mut parsed = Url::parse(connect_server_url)
        .map_err(|_| "connect: Connect Server URL is invalid".to_string())?;
    match parsed.scheme() {
        "http" => parsed
            .set_scheme("ws")
            .map_err(|_| "connect: failed to build websocket URL".to_string())?,
        "https" => parsed
            .set_scheme("wss")
            .map_err(|_| "connect: failed to build websocket URL".to_string())?,
        _ => return Err("connect: unsupported websocket URL scheme".to_string()),
    }
    parsed.set_path("/v1/ws");
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PlayingState, SongResponse};

    fn shared_playback(update_reason: Option<&str>) -> ConnectSharedPlaybackState {
        ConnectSharedPlaybackState {
            seq: 1,
            active_device_id: Some("device".to_string()),
            state: ConnectPlaybackState {
                playing_state: PlayingState::Stopped,
                queue: Vec::new(),
                current_index: None,
                play_next_queue_len: 0,
                current_position_ms: 0,
                current_song_id: None,
            },
            updated_at: "2026-05-25T00:00:00Z".to_string(),
            updated_by_device_id: Some("device".to_string()),
            update_reason: update_reason.map(ToOwned::to_owned),
        }
    }

    fn song(id: &str) -> SongResponse {
        SongResponse {
            id: id.to_string(),
            parent_id: None,
            path: None,
            title: id.to_string(),
            album: None,
            album_id: None,
            artist: None,
            artist_id: None,
            cover_art_id: None,
            display_cover_art_id: None,
            track: None,
            disc_number: None,
            year: None,
            duration: None,
            size: None,
            content_type: None,
            suffix: None,
            bit_rate: None,
            genre: None,
            created: None,
            starred: None,
            is_directory: false,
            media_type: None,
        }
    }

    fn transfer_validation_inner(active_device_id: Option<&str>) -> ConnectRuntimeInner {
        let mut shared = shared_playback(Some("snapshot"));
        shared.active_device_id = active_device_id.map(ToOwned::to_owned);
        shared.state.queue = vec![song("song-a")];
        ConnectRuntimeInner {
            status: ConnectRuntimeStatus {
                enabled: true,
                connected: true,
                ..ConnectRuntimeStatus::default()
            },
            devices: vec![ConnectDevicePresence {
                device_id: "target".to_string(),
                display_name: "Target".to_string(),
                platform: "test".to_string(),
                app_version: "0.0.0".to_string(),
                last_seen_at: "2026-05-25T00:00:00Z".to_string(),
                online: true,
            }],
            shared_playback: Some(shared),
            ..ConnectRuntimeInner::default()
        }
    }

    #[test]
    fn active_shared_snapshot_is_not_applied_to_backend() {
        let shared = shared_playback(Some(UPDATE_REASON_SNAPSHOT));

        assert!(!should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_SNAPSHOT,
            &shared,
            true,
            true,
            true,
        ));
    }

    #[test]
    fn active_shared_snapshot_is_applied_when_backend_is_uninitialized() {
        let shared = shared_playback(Some(UPDATE_REASON_SNAPSHOT));

        assert!(should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_SNAPSHOT,
            &shared,
            true,
            true,
            false,
        ));
    }

    #[test]
    fn non_active_shared_snapshot_is_applied_to_backend() {
        let shared = shared_playback(Some(UPDATE_REASON_SNAPSHOT));

        assert!(should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_SNAPSHOT,
            &shared,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn active_offline_update_is_not_applied_to_backend() {
        let shared = shared_playback(Some(UPDATE_REASON_ACTIVE_OFFLINE));

        assert!(!should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_UPDATED,
            &shared,
            true,
            true,
            false,
        ));
    }

    #[test]
    fn normal_shared_update_is_applied_when_runtime_allows() {
        let shared = shared_playback(Some("command"));

        assert!(should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_UPDATED,
            &shared,
            true,
            true,
            true,
        ));
    }

    #[test]
    fn transfer_shared_update_is_applied_when_runtime_allows() {
        let shared = shared_playback(Some("transfer"));

        assert!(should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_UPDATED,
            &shared,
            true,
            true,
            true,
        ));
    }

    #[test]
    fn runtime_skip_still_prevents_backend_apply() {
        let shared = shared_playback(Some("report"));

        assert!(!should_apply_shared_playback_to_backend(
            TYPE_PLAYBACK_SHARED_UPDATED,
            &shared,
            false,
            true,
            false,
        ));
    }

    #[test]
    fn transfer_validation_allows_missing_active_device() {
        let inner = transfer_validation_inner(None);

        assert!(ConnectRuntime::validate_transfer_playback_target(&inner, "target").is_ok());
    }

    #[test]
    fn transfer_validation_still_rejects_current_active_device() {
        let inner = transfer_validation_inner(Some("target"));

        assert_eq!(
            ConnectRuntime::validate_transfer_playback_target(&inner, "target"),
            Err("connect: target device is already active".to_string()),
        );
    }

    #[test]
    fn connect_restore_deferral_requires_enabled_connect_device_and_url() {
        let mut settings = ConnectSettings {
            enabled: true,
            device_id: "device-1".to_string(),
            allow_insecure_connect_server: true,
            ..ConnectSettings::default()
        };

        assert!(should_defer_local_playback_restore_for_connect(
            &settings,
            "http://subsonic.example:4533"
        ));

        settings.enabled = false;
        assert!(!should_defer_local_playback_restore_for_connect(
            &settings,
            "http://subsonic.example:4533"
        ));

        settings.enabled = true;
        settings.device_id.clear();
        assert!(!should_defer_local_playback_restore_for_connect(
            &settings,
            "http://subsonic.example:4533"
        ));
    }
}

fn current_connect_server_url(app: &tauri::AppHandle) -> Option<String> {
    let settings = app
        .state::<AppSettingsState>()
        .0
        .lock()
        .ok()
        .map(|guard| guard.snapshot().0.connect)?;
    let subsonic_server_url = app
        .state::<ActiveSessionState>()
        .0
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .map(|session| session.normalized_server_url.clone())
        });

    resolve_connect_server_url(&settings, subsonic_server_url.as_deref())
        .ok()
        .flatten()
}

fn connect_message_id(message_type: &str) -> String {
    format!(
        "{}-{}",
        message_type.replace('.', "-"),
        uuid::Uuid::new_v4()
    )
}
