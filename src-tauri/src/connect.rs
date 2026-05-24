use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
        AuthInput, ConnectDevicePresence, ConnectDeviceWithPlayback, ConnectDevicesUpdated,
        ConnectPlaybackDeviceState, ConnectPlaybackState, ConnectRuntimeStatus, ConnectSettings,
        PlaybackStatus,
    },
    playback::PlaybackRuntimeContext,
    ActiveSessionState, AppSettingsState, CoverArtCacheState, PlaybackControllerState,
};

const CONNECT_PROTOCOL_VERSION: u32 = 1;

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
    last_status: Option<ConnectPlaybackState>,
    devices: Vec<ConnectDevicePresence>,
    playback_by_device: BTreeMap<String, ConnectPlaybackDeviceState>,
}

enum ConnectOutbound {
    PlaybackState(ConnectPlaybackState),
    Envelope(ConnectOutboundEnvelope),
}

struct ConnectOutboundEnvelope {
    id: Option<String>,
    message_type: &'static str,
    payload: serde_json::Value,
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
        inner.status.connected = false;
        inner.status.message = Some("connect: restarting".to_string());
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
        inner.playback_by_device.clear();
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
        inner.playback_by_device.clear();
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
        let cached_status = {
            let mut inner = self.inner.lock().unwrap();
            if inner.generation != generation {
                return;
            }
            inner.sender = Some(sender.clone());
            inner.device_id = Some(device_id.clone());
            inner.status.enabled = true;
            inner.status.connected = true;
            inner.status.server_url = Some(server_url);
            inner.status.device_id = Some(device_id);
            inner.status.message = Some("connect: websocket connected".to_string());
            inner.last_status.clone()
        };
        if let Some(status) = cached_status {
            let _ = sender.send(ConnectOutbound::PlaybackState(status));
        }
    }

    fn next_seq(&self, generation: u64) -> Option<u32> {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return None;
        }
        let seq = inner.status.seq.saturating_add(1);
        inner.status.seq = seq;
        Some(seq)
    }

    fn publish_status(&self, status: PlaybackStatus) {
        let state = ConnectPlaybackState::from(status);
        let sender = {
            let mut inner = self.inner.lock().unwrap();
            inner.last_status = Some(state.clone());
            inner.sender.clone()
        };
        if let Some(sender) = sender {
            let _ = sender.send(ConnectOutbound::PlaybackState(state));
        }
    }

    pub(crate) fn request_handoff(&self, source_device_id: String) -> Result<(), String> {
        self.send_envelope(
            None,
            "handoff.request",
            serde_json::json!({ "sourceDeviceId": source_device_id }),
        )
    }

    pub(crate) fn pause_remote_device(&self, target_device_id: String) -> Result<(), String> {
        self.send_envelope(
            None,
            "remote.command",
            serde_json::json!({
                "targetDeviceId": target_device_id,
                "command": "pause",
            }),
        )
    }

    fn send_handoff_accept(
        &self,
        generation: u64,
        handoff_id: String,
        target_device_id: String,
        snapshot: ConnectPlaybackDeviceState,
    ) -> Result<(), String> {
        self.send_envelope(
            Some(generation),
            "handoff.accept",
            serde_json::json!({
                "handoffId": handoff_id,
                "targetDeviceId": target_device_id,
                "snapshot": snapshot,
            }),
        )
    }

    fn send_handoff_reject(
        &self,
        generation: u64,
        handoff_id: String,
        target_device_id: String,
        reason: String,
    ) -> Result<(), String> {
        self.send_envelope(
            Some(generation),
            "handoff.reject",
            serde_json::json!({
                "handoffId": handoff_id,
                "targetDeviceId": target_device_id,
                "reason": reason,
            }),
        )
    }

    fn send_handoff_complete(
        &self,
        generation: u64,
        handoff_id: String,
        source_device_id: String,
    ) -> Result<(), String> {
        self.send_envelope(
            Some(generation),
            "handoff.complete",
            serde_json::json!({
                "handoffId": handoff_id,
                "sourceDeviceId": source_device_id,
            }),
        )
    }

    fn send_remote_ack(&self, generation: u64, command_id: String) -> Result<(), String> {
        self.send_envelope(
            Some(generation),
            "remote.ack",
            serde_json::json!({ "commandId": command_id }),
        )
    }

    fn send_remote_error(
        &self,
        generation: u64,
        command_id: String,
        message: String,
    ) -> Result<(), String> {
        self.send_envelope(
            Some(generation),
            "remote.error",
            serde_json::json!({
                "commandId": command_id,
                "message": message,
            }),
        )
    }

    fn send_envelope(
        &self,
        generation: Option<u64>,
        message_type: &'static str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let sender = {
            let inner = self.inner.lock().unwrap();
            if generation.is_some_and(|expected| inner.generation != expected) {
                return Err("connect: runtime restarted".to_string());
            }
            inner.sender.clone()
        }
        .ok_or_else(|| "connect: websocket is not connected".to_string())?;

        sender
            .send(ConnectOutbound::Envelope(ConnectOutboundEnvelope {
                id: Some(connect_message_id(message_type)),
                message_type,
                payload,
            }))
            .map_err(|_| "connect: websocket writer is not available".to_string())
    }

    fn snapshot_for_state(
        &self,
        state: ConnectPlaybackState,
    ) -> Result<ConnectPlaybackDeviceState, String> {
        let inner = self.inner.lock().unwrap();
        let device_id = inner
            .device_id
            .clone()
            .ok_or_else(|| "connect: device id is unavailable".to_string())?;
        Ok(ConnectPlaybackDeviceState {
            seq: inner.status.seq.saturating_add(1),
            source_device_id: device_id,
            position_ms: state.current_position_ms,
            updated_at: connect_timestamp(),
            state,
        })
    }

    pub(crate) fn status(&self) -> ConnectRuntimeStatus {
        self.inner.lock().unwrap().status.clone()
    }

    pub(crate) fn devices_with_playback(&self) -> Vec<ConnectDeviceWithPlayback> {
        let inner = self.inner.lock().unwrap();
        inner.devices_with_playback()
    }

    fn set_devices(
        &self,
        generation: u64,
        devices: Vec<ConnectDevicePresence>,
    ) -> Option<Vec<ConnectDeviceWithPlayback>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return None;
        }
        inner.devices = devices;
        Some(inner.devices_with_playback())
    }

    fn upsert_playback(
        &self,
        generation: u64,
        snapshot: ConnectPlaybackDeviceState,
        republish_after_own_snapshot: bool,
    ) -> Option<Vec<ConnectDeviceWithPlayback>> {
        let (devices, republish) = {
            let mut inner = self.inner.lock().unwrap();
            if inner.generation != generation {
                return None;
            }
            let is_own_snapshot = inner
                .device_id
                .as_deref()
                .is_some_and(|device_id| device_id == snapshot.source_device_id);
            let snapshot_seq = snapshot.seq;
            let current = inner
                .playback_by_device
                .get(&snapshot.source_device_id)
                .map(|state| state.seq)
                .unwrap_or(0);
            if snapshot_seq >= current {
                inner
                    .playback_by_device
                    .insert(snapshot.source_device_id.clone(), snapshot);
            }
            let own_seq_advanced = is_own_snapshot && snapshot_seq >= inner.status.seq;
            if own_seq_advanced {
                inner.status.seq = snapshot_seq;
            }
            let republish = if republish_after_own_snapshot && own_seq_advanced {
                inner.sender.clone().zip(inner.last_status.clone())
            } else {
                None
            };
            (inner.devices_with_playback(), republish)
        };
        if let Some((sender, status)) = republish {
            let _ = sender.send(ConnectOutbound::PlaybackState(status));
        }
        Some(devices)
    }
}

impl ConnectRuntimeInner {
    fn devices_with_playback(&self) -> Vec<ConnectDeviceWithPlayback> {
        self.devices
            .iter()
            .cloned()
            .map(|device| {
                let playback = self.playback_by_device.get(&device.device_id).cloned();
                ConnectDeviceWithPlayback { device, playback }
            })
            .collect()
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
        let settings = settings_state
            .0
            .lock()
            .map_err(|_| "connect: settings state unavailable".to_string())?
            .snapshot()
            .0;
        settings
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
            Some(outbound) = receiver.recv() => {
                match outbound {
                    ConnectOutbound::PlaybackState(state) => {
                        let Some(seq) = runtime.next_seq(generation) else {
                            return Ok(ConnectRunResult::Restart);
                        };
                        let envelope = ConnectEnvelope {
                            id: Some(format!("playback-{seq}")),
                            message_type: "playback.state.publish",
                            protocol_version: CONNECT_PROTOCOL_VERSION,
                            device_id: &login_response.device_id,
                            payload: PlaybackPublishPayload {
                                seq,
                                state,
                            },
                        };
                        write_json_message(&mut write, &envelope).await?;
                    }
                    ConnectOutbound::Envelope(outbound) => {
                        let envelope = ConnectEnvelope {
                            id: outbound.id,
                            message_type: outbound.message_type,
                            protocol_version: CONNECT_PROTOCOL_VERSION,
                            device_id: &login_response.device_id,
                            payload: outbound.payload,
                        };
                        write_json_message(&mut write, &envelope).await?;
                    }
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
                        handle_incoming_message(app, runtime.as_ref(), generation, message).await?;
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
    runtime: &ConnectRuntime,
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
        "playback.state.snapshot" => {
            for snapshot in playback_snapshots_from_payload(payload)? {
                if let Some(devices) = runtime.upsert_playback(generation, snapshot, true) {
                    emit_devices_updated(app, devices);
                }
            }
        }
        "playback.state.updated" => {
            let snapshot: ConnectPlaybackDeviceState = serde_json::from_value(payload)
                .map_err(|error| format!("connect: playback update decode failed: {error}"))?;
            if let Some(devices) = runtime.upsert_playback(generation, snapshot, false) {
                emit_devices_updated(app, devices);
            }
        }
        "handoff.request" => {
            handle_handoff_request(app.clone(), runtime, generation, payload).await?;
        }
        "handoff.accept" => {
            handle_handoff_accept(app.clone(), runtime, generation, payload).await?;
        }
        "handoff.reject" => {
            let payload: HandoffRejectPayload = serde_json::from_value(payload)
                .map_err(|error| format!("connect: handoff reject decode failed: {error}"))?;
            log::warn!(
                "connect: handoff rejected handoff_id={} reason={}",
                payload.handoff_id,
                payload.reason.unwrap_or_default()
            );
        }
        "handoff.complete" => {
            handle_handoff_complete(app.clone(), payload).await?;
        }
        "remote.command" => {
            handle_remote_command(app.clone(), runtime, generation, payload).await?;
        }
        "remote.error" => {
            let payload: RemoteErrorPayload = serde_json::from_value(payload)
                .map_err(|error| format!("connect: remote error decode failed: {error}"))?;
            log::warn!(
                "connect: remote command failed command_id={} message={}",
                payload.command_id,
                payload.message.unwrap_or_default()
            );
        }
        "remote.ack" => {}
        _ => {}
    }

    Ok(())
}

async fn handle_handoff_request(
    app: tauri::AppHandle,
    runtime: &ConnectRuntime,
    generation: u64,
    payload: serde_json::Value,
) -> Result<(), String> {
    let payload: HandoffRequestPayload = serde_json::from_value(payload)
        .map_err(|error| format!("connect: handoff request decode failed: {error}"))?;

    let status = match current_playback_status(app).await {
        Ok(status) => status,
        Err(error) => {
            let _ = runtime.send_handoff_reject(
                generation,
                payload.handoff_id,
                payload.target_device_id,
                error,
            );
            return Ok(());
        }
    };

    let snapshot = runtime.snapshot_for_state(ConnectPlaybackState::from(status))?;
    runtime.send_handoff_accept(
        generation,
        payload.handoff_id,
        payload.target_device_id,
        snapshot,
    )?;
    Ok(())
}

async fn handle_handoff_accept(
    app: tauri::AppHandle,
    runtime: &ConnectRuntime,
    generation: u64,
    payload: serde_json::Value,
) -> Result<(), String> {
    let payload: HandoffAcceptPayload = serde_json::from_value(payload)
        .map_err(|error| format!("connect: handoff accept decode failed: {error}"))?;

    if let Some(snapshot) = payload.snapshot {
        let snapshot = decode_handoff_snapshot(snapshot)?;
        apply_takeover_snapshot(app, snapshot).await?;
    }

    runtime.send_handoff_complete(generation, payload.handoff_id, payload.source_device_id)?;
    Ok(())
}

async fn handle_handoff_complete(
    app: tauri::AppHandle,
    payload: serde_json::Value,
) -> Result<(), String> {
    let payload: HandoffCompletePayload = serde_json::from_value(payload)
        .map_err(|error| format!("connect: handoff complete decode failed: {error}"))?;
    if let Some(snapshot) = payload.snapshot {
        let snapshot = decode_handoff_snapshot(snapshot)?;
        apply_takeover_snapshot(app, snapshot).await?;
    }
    Ok(())
}

async fn handle_remote_command(
    app: tauri::AppHandle,
    runtime: &ConnectRuntime,
    generation: u64,
    payload: serde_json::Value,
) -> Result<(), String> {
    let payload: RemoteCommandPayload = serde_json::from_value(payload)
        .map_err(|error| format!("connect: remote command decode failed: {error}"))?;

    match execute_remote_playback_command(app, &payload.command).await {
        Ok(()) => runtime.send_remote_ack(generation, payload.command_id)?,
        Err(error) => runtime.send_remote_error(generation, payload.command_id, error)?,
    }
    Ok(())
}

async fn current_playback_status(app: tauri::AppHandle) -> Result<PlaybackStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let sessions = app.state::<ActiveSessionState>();
        let cover_art_cache = app.state::<CoverArtCacheState>();
        let playback = app.state::<PlaybackControllerState>();
        let context_parts = active_runtime_parts(&app, &sessions);
        let mut controller = playback
            .0
            .lock()
            .map_err(|_| "The playback controller state is unavailable.".to_string())?;

        match context_parts {
            Ok((ref client, ref capability_matrix, ref profile_id)) => {
                let runtime_context = PlaybackRuntimeContext {
                    client,
                    capability_matrix,
                    cover_art_cache: Some(&cover_art_cache.0),
                    profile_id: Some(profile_id.as_str()),
                };
                controller.synced_state_with_context(Some(&runtime_context))
            }
            Err(_) => controller.synced_state(),
        }
    })
    .await
    .map_err(|error| format!("connect: playback status task failed: {error}"))?
}

async fn apply_takeover_snapshot(
    app: tauri::AppHandle,
    snapshot: ConnectPlaybackDeviceState,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let sessions = app.state::<ActiveSessionState>();
        let cover_art_cache = app.state::<CoverArtCacheState>();
        let playback = app.state::<PlaybackControllerState>();
        let (active_client, active_capability_matrix, active_profile_id) =
            active_runtime_parts(&app, &sessions)?;

        let mut state = snapshot.state;
        let current_index = takeover_current_index(&state)
            .ok_or_else(|| "connect: source playback queue is empty".to_string())?;
        let queue = std::mem::take(&mut state.queue);
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
        controller.set_queue(queue, Some(current_index))?;
        controller.set_position(snapshot.position_ms)?;
        controller.play(&runtime_context).map(|_| ())
    })
    .await
    .map_err(|error| format!("connect: takeover task failed: {error}"))?
}

async fn execute_remote_playback_command(
    app: tauri::AppHandle,
    command: &str,
) -> Result<(), String> {
    let command = command.to_string();
    tauri::async_runtime::spawn_blocking(move || {
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

        match (command.as_str(), runtime_context.as_ref()) {
            ("pause", Some((client, capability_matrix, profile_id, cover_art_cache))) => {
                let runtime_context = PlaybackRuntimeContext {
                    client,
                    capability_matrix,
                    cover_art_cache: Some(cover_art_cache),
                    profile_id: Some(profile_id),
                };
                controller
                    .pause_with_context(Some(&runtime_context))
                    .map(|_| ())
            }
            ("pause", None) => controller.pause().map(|_| ()),
            ("stop", Some((client, capability_matrix, profile_id, cover_art_cache))) => {
                let runtime_context = PlaybackRuntimeContext {
                    client,
                    capability_matrix,
                    cover_art_cache: Some(cover_art_cache),
                    profile_id: Some(profile_id),
                };
                controller
                    .stop_with_context(Some(&runtime_context))
                    .map(|_| ())
            }
            ("stop", None) => controller.stop().map(|_| ()),
            _ => Err(format!("connect: unsupported remote command {command:?}")),
        }
    })
    .await
    .map_err(|error| format!("connect: remote command task failed: {error}"))?
}

fn takeover_current_index(state: &ConnectPlaybackState) -> Option<u32> {
    if state.queue.is_empty() {
        return None;
    }
    if let Some(index) = state.current_index {
        if (index as usize) < state.queue.len() {
            return Some(index);
        }
    }
    if let Some(current_song_id) = state.current_song_id.as_deref() {
        if let Some(index) = state
            .queue
            .iter()
            .position(|song| song.id == current_song_id)
            .and_then(|index| u32::try_from(index).ok())
        {
            return Some(index);
        }
    }
    Some(0)
}

fn decode_handoff_snapshot(value: serde_json::Value) -> Result<ConnectPlaybackDeviceState, String> {
    match serde_json::from_value::<ConnectPlaybackDeviceState>(value.clone()) {
        Ok(snapshot) => Ok(snapshot),
        Err(_) => {
            let snapshot: StoredPlaybackSnapshotPayload = serde_json::from_value(value)
                .map_err(|error| format!("connect: handoff snapshot decode failed: {error}"))?;
            Ok(ConnectPlaybackDeviceState {
                seq: snapshot.seq,
                source_device_id: snapshot.source_device_id,
                position_ms: snapshot.current_position_ms,
                updated_at: snapshot.updated_at,
                state: snapshot.state,
            })
        }
    }
}

fn playback_snapshots_from_payload(
    payload: serde_json::Value,
) -> Result<Vec<ConnectPlaybackDeviceState>, String> {
    if payload.get("snapshots").is_some() {
        let payload: PlaybackSnapshotsPayload = serde_json::from_value(payload)
            .map_err(|error| format!("connect: playback snapshots decode failed: {error}"))?;
        return Ok(payload.snapshots);
    }

    if payload.get("snapshot").is_some() {
        let payload: PlaybackSnapshotWrapper = serde_json::from_value(payload)
            .map_err(|error| format!("connect: playback snapshot decode failed: {error}"))?;
        return Ok(payload.snapshot.into_iter().collect());
    }

    let snapshot: ConnectPlaybackDeviceState = serde_json::from_value(payload)
        .map_err(|error| format!("connect: playback snapshot decode failed: {error}"))?;
    Ok(vec![snapshot])
}

fn emit_devices_updated(app: &tauri::AppHandle, devices: Vec<ConnectDeviceWithPlayback>) {
    if let Err(error) = (ConnectDevicesUpdated { devices }).emit(app) {
        log::warn!("connect: failed to emit devices update: {error}");
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
struct PlaybackPublishPayload {
    seq: u32,
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
struct PlaybackSnapshotsPayload {
    snapshots: Vec<ConnectPlaybackDeviceState>,
}

#[derive(Debug, Deserialize)]
struct PlaybackSnapshotWrapper {
    snapshot: Option<ConnectPlaybackDeviceState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffRequestPayload {
    handoff_id: String,
    target_device_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffAcceptPayload {
    handoff_id: String,
    source_device_id: String,
    snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffRejectPayload {
    handoff_id: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffCompletePayload {
    snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredPlaybackSnapshotPayload {
    seq: u32,
    source_device_id: String,
    state: ConnectPlaybackState,
    current_position_ms: u32,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteCommandPayload {
    command_id: String,
    command: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteErrorPayload {
    command_id: String,
    message: Option<String>,
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

fn connect_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
