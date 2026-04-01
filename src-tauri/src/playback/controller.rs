use opensubsonic_client::{
    api::retrieval::{RetrievalApi, StreamRequest},
    ApiError, OpenSubsonicClient, PreparedBinaryRequest,
};

use crate::models::{
    CapabilityMatrix, PlaybackQueueEntry, PlaybackSetQueueRequest, PlaybackState, PlaybackStatus,
};

use super::{
    backend::{create_playback_backend, PlaybackBackend},
    queue_sync::{NoopQueueSyncGateway, QueueSyncGateway},
    reporting::{NoopPlaybackReporter, PlaybackReporter},
};

pub struct PlaybackRuntimeContext<'a> {
    pub client: &'a OpenSubsonicClient,
    pub capability_matrix: &'a CapabilityMatrix,
}

pub struct PlaybackController {
    backend: Box<dyn PlaybackBackend>,
    reporter: Box<dyn PlaybackReporter>,
    queue_sync: Box<dyn QueueSyncGateway>,
    status: PlaybackStatus,
}

impl PlaybackController {
    pub fn with_defaults() -> Self {
        Self::new(
            create_playback_backend(),
            Box::new(NoopPlaybackReporter),
            Box::new(NoopQueueSyncGateway),
        )
    }

    pub fn new(
        backend: Box<dyn PlaybackBackend>,
        reporter: Box<dyn PlaybackReporter>,
        queue_sync: Box<dyn QueueSyncGateway>,
    ) -> Self {
        Self {
            backend,
            reporter,
            queue_sync,
            status: PlaybackStatus::empty(),
        }
    }

    pub fn state(&self) -> PlaybackStatus {
        self.status.clone()
    }

    pub fn set_queue(
        &mut self,
        payload: PlaybackSetQueueRequest,
    ) -> Result<PlaybackStatus, String> {
        validate_queue_index(&payload.entries, payload.current_index)?;
        self.backend.stop()?;

        let next_song_id = current_song_id(&payload.entries, payload.current_index);
        self.status.queue = payload.entries;
        self.status.current_index = payload.current_index;
        self.status.current_position_ms = 0;
        self.status.current_song_id = next_song_id;
        self.status.error = None;
        self.status.state = if self.status.queue.is_empty() {
            PlaybackState::Idle
        } else {
            PlaybackState::Stopped
        };

        let _ = self.queue_sync.sync_queue(&self.status);
        Ok(self.state())
    }

    pub fn play(&mut self, context: &PlaybackRuntimeContext<'_>) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        let requested_position_ms = self.status.current_position_ms;
        self.load_track_at_index(context, index, requested_position_ms, true)?;
        let _ = self.reporter.report_state(&self.status);
        Ok(self.state())
    }

    pub fn pause(&mut self) -> Result<PlaybackStatus, String> {
        self.backend.pause()?;
        self.status.state = PlaybackState::Paused;
        self.status.error = None;
        let _ = self.reporter.report_state(&self.status);
        Ok(self.state())
    }

    pub fn stop(&mut self) -> Result<PlaybackStatus, String> {
        self.backend.stop()?;
        self.status.state = PlaybackState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        let _ = self.reporter.report_state(&self.status);
        Ok(self.state())
    }

    pub fn seek(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        requested_position_ms: u32,
    ) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        let supports_offset = context.capability_matrix.transcode_offset;
        let normalized_position_ms =
            normalize_seek_position(requested_position_ms, supports_offset);
        let autoplay = matches!(self.status.state, PlaybackState::Playing);

        if matches!(
            self.status.state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            self.load_track_at_index(context, index, normalized_position_ms, autoplay)?;
        } else {
            self.status.current_position_ms = normalized_position_ms;
            self.status.error = None;
        }

        let _ = self.reporter.report_state(&self.status);
        Ok(self.state())
    }

    pub fn next(&mut self, context: &PlaybackRuntimeContext<'_>) -> Result<PlaybackStatus, String> {
        let current_index = self.ensure_current_index()?;
        let Some(next_index) = current_index.checked_add(1) else {
            return Err("The current queue position is out of range.".to_string());
        };
        if match usize::try_from(next_index).ok() {
            Some(idx) => idx >= self.status.queue.len(),
            None => true,
        } {
            return Err("Already at the end of the playback queue.".to_string());
        }

        let autoplay = matches!(self.status.state, PlaybackState::Playing);
        if matches!(
            self.status.state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            self.load_track_at_index(context, next_index, 0, autoplay)?;
        } else {
            self.status.current_index = Some(next_index);
            self.status.current_song_id = current_song_id(&self.status.queue, Some(next_index));
            self.status.current_position_ms = 0;
            self.status.state = PlaybackState::Stopped;
            self.status.error = None;
        }

        let _ = self.reporter.report_state(&self.status);
        Ok(self.state())
    }

    pub fn prev(&mut self, context: &PlaybackRuntimeContext<'_>) -> Result<PlaybackStatus, String> {
        let current_index = self.ensure_current_index()?;
        if current_index == 0 {
            return Err("Already at the beginning of the playback queue.".to_string());
        }

        let prev_index = current_index - 1;
        let autoplay = matches!(self.status.state, PlaybackState::Playing);
        if matches!(
            self.status.state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            self.load_track_at_index(context, prev_index, 0, autoplay)?;
        } else {
            self.status.current_index = Some(prev_index);
            self.status.current_song_id = current_song_id(&self.status.queue, Some(prev_index));
            self.status.current_position_ms = 0;
            self.status.state = PlaybackState::Stopped;
            self.status.error = None;
        }

        let _ = self.reporter.report_state(&self.status);
        Ok(self.state())
    }

    #[allow(dead_code)]
    pub fn on_track_finished(&mut self) -> PlaybackStatus {
        self.status.state = PlaybackState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        let _ = self.reporter.report_state(&self.status);
        self.state()
    }

    fn ensure_current_index(&mut self) -> Result<u32, String> {
        if self.status.queue.is_empty() {
            log::warn!("controller.ensure_current_index: queue is empty");
            return Err("Playback queue is empty.".to_string());
        }

        if let Some(index) = self.status.current_index {
            if match usize::try_from(index).ok() {
                Some(idx) => idx < self.status.queue.len(),
                None => false,
            } {
                return Ok(index);
            }
            log::warn!(
                "controller.ensure_current_index: out of range index={index} queue_len={}",
                self.status.queue.len()
            );
            return Err("Current playback index is out of range.".to_string());
        }

        self.status.current_index = Some(0);
        self.status.current_song_id = current_song_id(&self.status.queue, Some(0));
        Ok(0)
    }

    fn load_track_at_index(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        index: u32,
        requested_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String> {
        let Some(song_id) = current_song_id(&self.status.queue, Some(index)) else {
            return Err("Current playback entry does not exist.".to_string());
        };

        let supports_offset = context.capability_matrix.transcode_offset;
        let normalized_position_ms =
            normalize_seek_position(requested_position_ms, supports_offset);
        let stream_offset_seconds = position_ms_to_offset_seconds(normalized_position_ms);
        self.status.state = PlaybackState::Loading;
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id.clone());
        self.status.current_position_ms = normalized_position_ms;
        self.status.error = None;

        let raw_stream =
            build_stream_request(context.client, &song_id, stream_offset_seconds, true)?;
        if let Err(raw_error) = self.backend.load(raw_stream, autoplay) {
            log::warn!(
                "controller.load_track_at_index: raw stream load failed for song_id={song_id}: {raw_error}"
            );
            let fallback_stream =
                build_stream_request(context.client, &song_id, stream_offset_seconds, false)?;
            if let Err(fallback_error) = self.backend.load(fallback_stream, autoplay) {
                let message = format!(
                    "Failed to load playback stream. raw stream failed: {raw_error}; fallback stream failed: {fallback_error}"
                );
                log::error!(
                    "controller.load_track_at_index: fallback stream load failed for song_id={song_id}: {fallback_error}"
                );
                self.status.state = PlaybackState::Error;
                self.status.error = Some(message.clone());
                return Err(message);
            }
        }

        self.status.state = if autoplay {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        };
        self.status.current_position_ms = normalized_position_ms;
        self.status.current_song_id = Some(song_id);
        self.status.error = None;

        Ok(())
    }
}

fn build_stream_request(
    client: &OpenSubsonicClient,
    song_id: &str,
    stream_offset_seconds: Option<u32>,
    raw_stream: bool,
) -> Result<PreparedBinaryRequest, String> {
    client
        .stream(StreamRequest {
            id: song_id.to_string(),
            max_bit_rate: None,
            format: raw_stream.then_some("raw".to_string()),
            time_offset: stream_offset_seconds,
            size: None,
            estimate_content_length: None,
            converted: None,
        })
        .map_err(format_api_error)
}

fn validate_queue_index(
    entries: &[PlaybackQueueEntry],
    current_index: Option<u32>,
) -> Result<(), String> {
    if entries.is_empty() {
        if current_index.is_some() {
            return Err("currentIndex must be omitted when entries is empty.".to_string());
        }
        return Ok(());
    }

    let Some(current_index) = current_index else {
        return Ok(());
    };
    if match usize::try_from(current_index).ok() {
        Some(index) => index >= entries.len(),
        None => true,
    } {
        return Err("currentIndex is out of range for the provided queue entries.".to_string());
    }

    Ok(())
}

fn current_song_id(entries: &[PlaybackQueueEntry], current_index: Option<u32>) -> Option<String> {
    let index = usize::try_from(current_index?).ok()?;
    entries.get(index).map(|entry| entry.song_id.clone())
}

fn normalize_seek_position(position_ms: u32, supports_offset: bool) -> u32 {
    if supports_offset {
        position_ms
    } else {
        0
    }
}

fn position_ms_to_offset_seconds(position_ms: u32) -> Option<u32> {
    if position_ms == 0 {
        return None;
    }

    Some(position_ms / 1000)
}

fn format_api_error(error: ApiError) -> String {
    match error {
        ApiError::InvalidUrl(message)
        | ApiError::ClientBuild(message)
        | ApiError::Protocol(message) => message,
        ApiError::Transport(error) => error.to_string(),
        ApiError::HttpStatus { status, .. } => format!("HTTP {}", status.as_u16()),
        ApiError::Decode { message, .. } => message,
        ApiError::Api { code, message, .. } => {
            let detail = message.unwrap_or_else(|| "Unknown API error".to_string());
            format!("API {code}: {detail}")
        }
        ApiError::UnsupportedExtension { extension } => {
            format!("Missing extension: {extension}")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use opensubsonic_client::{normalize_base_url, Auth, ClientConfig};

    use super::{
        current_song_id, normalize_seek_position, position_ms_to_offset_seconds,
        PlaybackController, PlaybackRuntimeContext, PlaybackStatus,
    };
    use crate::{
        models::{CapabilityMatrix, PlaybackQueueEntry, PlaybackSetQueueRequest, PlaybackState},
        playback::{
            backend::PlaybackBackend,
            queue_sync::{NoopQueueSyncGateway, QueueSyncGateway},
            reporting::{NoopPlaybackReporter, PlaybackReporter},
        },
    };

    #[derive(Debug, Default)]
    struct MockBackendState {
        load_calls: Vec<String>,
        fail_first_load: bool,
        has_failed_first_load: bool,
        pause_calls: usize,
        stop_calls: usize,
    }

    struct MockBackend {
        state: Arc<Mutex<MockBackendState>>,
    }

    impl PlaybackBackend for MockBackend {
        fn load(
            &mut self,
            request: opensubsonic_client::PreparedBinaryRequest,
            _autoplay: bool,
        ) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.load_calls.push(request.url.to_string());
            if state.fail_first_load && !state.has_failed_first_load {
                state.has_failed_first_load = true;
                return Err("raw stream rejected".to_string());
            }

            Ok(())
        }

        fn pause(&mut self) -> Result<(), String> {
            self.state.lock().unwrap().pause_calls += 1;
            Ok(())
        }

        fn stop(&mut self) -> Result<(), String> {
            self.state.lock().unwrap().stop_calls += 1;
            Ok(())
        }
    }

    fn runtime_context(
        transcode_offset: bool,
    ) -> (opensubsonic_client::OpenSubsonicClient, CapabilityMatrix) {
        let mut capability_matrix = CapabilityMatrix::empty();
        capability_matrix.transcode_offset = transcode_offset;
        let client = opensubsonic_client::OpenSubsonicClient::new(ClientConfig::new(
            normalize_base_url("https://demo.example").unwrap(),
            Auth::Token {
                username: "demo".to_string(),
                password: "secret".to_string(),
            },
            "transonic",
        ))
        .unwrap();

        (client, capability_matrix)
    }

    fn controller_with_mock_backend(
        fail_first_load: bool,
    ) -> (PlaybackController, Arc<Mutex<MockBackendState>>) {
        let state = Arc::new(Mutex::new(MockBackendState {
            fail_first_load,
            ..MockBackendState::default()
        }));
        let backend = Box::new(MockBackend {
            state: state.clone(),
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(NoopPlaybackReporter);
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let controller = PlaybackController::new(backend, reporter, queue_sync);
        (controller, state)
    }

    fn queue_entries(song_ids: &[&str]) -> Vec<PlaybackQueueEntry> {
        song_ids
            .iter()
            .map(|song_id| PlaybackQueueEntry {
                song_id: (*song_id).to_string(),
            })
            .collect()
    }

    #[test]
    fn set_queue_rejects_out_of_range_current_index() {
        let (mut controller, _) = controller_with_mock_backend(false);

        let result = controller.set_queue(PlaybackSetQueueRequest {
            entries: queue_entries(&["song-a"]),
            current_index: Some(1),
        });

        assert!(result.is_err());
    }

    #[test]
    fn duplicate_song_ids_are_distinguished_by_index() {
        let (mut controller, _) = controller_with_mock_backend(false);
        let status = controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a", "song-a", "song-b"]),
                current_index: Some(1),
            })
            .unwrap();

        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-a"));
    }

    #[test]
    fn next_prev_enforce_queue_boundaries() {
        let (mut controller, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a", "song-b"]),
                current_index: Some(0),
            })
            .unwrap();

        let prev_result = controller.prev(&runtime_context);
        assert!(prev_result.is_err());

        controller.next(&runtime_context).unwrap();
        let next_result = controller.next(&runtime_context);
        assert!(next_result.is_err());
    }

    #[test]
    fn play_pause_seek_stop_follow_consistent_state_transitions() {
        let (mut controller, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();

        assert_eq!(
            controller.play(&runtime_context).unwrap().state,
            PlaybackState::Playing
        );
        assert_eq!(controller.pause().unwrap().state, PlaybackState::Paused);
        let seeked = controller.seek(&runtime_context, 8_000).unwrap();
        assert_eq!(seeked.state, PlaybackState::Paused);
        assert_eq!(seeked.current_position_ms, 8_000);
        assert_eq!(controller.stop().unwrap().state, PlaybackState::Stopped);
    }

    #[test]
    fn on_track_finished_does_not_auto_advance_index() {
        let (mut controller, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a", "song-b"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        let status = controller.on_track_finished();
        assert_eq!(status.state, PlaybackState::Stopped);
        assert_eq!(status.current_index, Some(0));
    }

    #[test]
    fn raw_stream_falls_back_to_standard_stream_when_backend_rejects_raw() {
        let (mut controller, backend_state) = controller_with_mock_backend(true);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 2);
        assert!(load_calls[0].contains("format=raw"));
        assert!(!load_calls[1].contains("format=raw"));
    }

    #[test]
    fn stream_offset_is_not_sent_when_transcode_offset_extension_is_missing() {
        let (mut controller, backend_state) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(false);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        let seeked = controller.seek(&runtime_context, 5_000).unwrap();
        assert_eq!(seeked.current_position_ms, 0);
        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(!load_calls[0].contains("timeOffset="));
    }

    #[test]
    fn helper_current_song_id_uses_index() {
        let entries = queue_entries(&["song-a", "song-a"]);
        assert_eq!(
            current_song_id(&entries, Some(1)).as_deref(),
            Some("song-a")
        );
        assert_eq!(current_song_id(&entries, Some(2)), None);
    }

    #[test]
    fn helper_normalize_seek_position_applies_capability_gate() {
        assert_eq!(normalize_seek_position(7_000, true), 7_000);
        assert_eq!(normalize_seek_position(7_000, false), 0);
    }

    #[test]
    fn helper_position_ms_to_offset_seconds_rounds_down() {
        assert_eq!(position_ms_to_offset_seconds(0), None);
        assert_eq!(position_ms_to_offset_seconds(1_999), Some(1));
    }

    #[test]
    fn default_status_is_idle() {
        assert_eq!(PlaybackStatus::empty().state, PlaybackState::Idle);
    }
}
