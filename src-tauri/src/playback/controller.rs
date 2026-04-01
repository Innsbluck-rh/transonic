use opensubsonic_client::{
    api::retrieval::{RetrievalApi, StreamRequest},
    ApiError, OpenSubsonicClient, PreparedBinaryRequest,
};

use crate::models::{
    CapabilityMatrix, PlaybackLoadingReason, PlaybackQueueEntry, PlaybackSetQueueRequest,
    PlaybackState, PlaybackStatus,
};

use super::{
    backend_shims::{create_playback_backend, PlaybackBackend},
    queue_sync::{NoopQueueSyncGateway, QueueSyncGateway},
    reporting::{PlaybackEventAppHandle, PlaybackReporter, TauriPlaybackReporter},
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
    current_source_absolute_start_position_ms: Option<u32>,
}

impl PlaybackController {
    pub(crate) fn with_tauri_reporter(app_handle: PlaybackEventAppHandle) -> Self {
        Self::new(
            create_playback_backend(),
            Box::new(TauriPlaybackReporter::new(app_handle)),
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
            current_source_absolute_start_position_ms: None,
        }
    }

    pub fn state(&self) -> PlaybackStatus {
        self.status.clone()
    }

    pub fn synced_state(&mut self) -> Result<PlaybackStatus, String> {
        self.sync_current_position_from_backend()?;
        Ok(self.state())
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
        self.status.loading_reason = None;
        self.status.state = if self.status.queue.is_empty() {
            PlaybackState::Idle
        } else {
            PlaybackState::Stopped
        };
        self.current_source_absolute_start_position_ms = None;

        if let Some(entry) = current_queue_entry(&self.status.queue, self.status.current_index) {
            log::info!(
                "controller.set_queue: current_index={:?} song_id={} path={:?} is_flac={}",
                self.status.current_index,
                entry.song_id,
                entry.path,
                is_probably_flac_path(entry.path.as_deref())
            );
        } else {
            log::info!(
                "controller.set_queue: current_index={:?} queue_len={} (no active entry)",
                self.status.current_index,
                self.status.queue.len()
            );
        }

        self.sync_queue_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn play(&mut self, context: &PlaybackRuntimeContext<'_>) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        let requested_position_ms = self.status.current_position_ms;
        self.load_track_at_index(context, index, requested_position_ms, true)?;
        self.report_status();
        Ok(self.state())
    }

    pub fn pause(&mut self) -> Result<PlaybackStatus, String> {
        self.sync_current_position_from_backend()?;
        self.backend.pause()?;
        self.status.state = PlaybackState::Paused;
        self.status.error = None;
        self.status.loading_reason = None;
        self.report_status();
        Ok(self.state())
    }

    pub fn stop(&mut self) -> Result<PlaybackStatus, String> {
        self.backend.stop()?;
        self.status.state = PlaybackState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.status.loading_reason = None;
        self.current_source_absolute_start_position_ms = None;
        self.report_status();
        Ok(self.state())
    }

    pub fn seek(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        requested_position_ms: u32,
    ) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        let autoplay = matches!(self.status.state, PlaybackState::Playing);
        let entry = self
            .status
            .queue
            .get(
                usize::try_from(index)
                    .map_err(|_| "Current playback index is out of range.".to_string())?,
            )
            .cloned();
        let is_flac = entry
            .as_ref()
            .is_some_and(|queue_entry| is_probably_flac_path(queue_entry.path.as_deref()));

        log::info!(
            "controller.seek: song_id={:?} path={:?} state={:?} requested_position_ms={} current_position_ms={} source_start_ms={:?} is_flac={}",
            entry.as_ref().map(|queue_entry| queue_entry.song_id.as_str()),
            entry.as_ref().and_then(|queue_entry| queue_entry.path.as_deref()),
            self.status.state,
            requested_position_ms,
            self.status.current_position_ms,
            self.current_source_absolute_start_position_ms,
            is_flac
        );

        if matches!(
            self.status.state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            self.sync_current_position_from_backend()?;

            if is_flac {
                log::info!(
                    "controller.seek: using exact reload because the current track is detected as FLAC"
                );
                self.reload_track_at_index_exact(context, index, requested_position_ms, autoplay)?;
            } else if self.current_source_absolute_start_position_ms == Some(0) {
                log::info!(
                    "controller.seek: attempting native backend seek because source_start_ms is zero and track is not detected as FLAC"
                );
                if let Err(error) = self.backend.seek(requested_position_ms) {
                    log::warn!(
                        "controller.seek: backend seek failed at position_ms={requested_position_ms}: {error}; song_id={:?} path={:?} is_flac={}; falling back to exact reload",
                        entry.as_ref().map(|queue_entry| queue_entry.song_id.as_str()),
                        entry.as_ref().and_then(|queue_entry| queue_entry.path.as_deref()),
                        is_flac
                    );
                    self.reload_track_at_index_exact(
                        context,
                        index,
                        requested_position_ms,
                        autoplay,
                    )?;
                } else {
                    self.status.current_position_ms = requested_position_ms;
                    self.status.error = None;
                    self.status.loading_reason = None;
                }
            } else {
                log::info!(
                    "controller.seek: using exact reload because source_start_ms={:?} indicates a non-zero loaded source",
                    self.current_source_absolute_start_position_ms
                );
                self.reload_track_at_index_exact(context, index, requested_position_ms, autoplay)?;
            }
        } else {
            self.status.current_position_ms = requested_position_ms;
            self.status.error = None;
            self.status.loading_reason = None;
        }

        self.report_status();
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
            self.status.loading_reason = None;
            self.current_source_absolute_start_position_ms = None;
        }

        self.report_status();
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
            self.status.loading_reason = None;
            self.current_source_absolute_start_position_ms = None;
        }

        self.report_status();
        Ok(self.state())
    }

    #[allow(dead_code)]
    pub fn on_track_finished(&mut self) -> PlaybackStatus {
        self.status.state = PlaybackState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.status.loading_reason = None;
        self.current_source_absolute_start_position_ms = None;
        self.report_status();
        self.state()
    }

    pub fn reset(&mut self) -> PlaybackStatus {
        if let Err(error) = self.backend.stop() {
            log::warn!("controller.reset: failed to stop backend: {error}");
        }

        self.status = PlaybackStatus::empty();
        self.current_source_absolute_start_position_ms = None;
        self.sync_queue_state();
        self.report_status();
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
        let Some(entry) = current_queue_entry(&self.status.queue, Some(index)).cloned() else {
            return Err("Current playback entry does not exist.".to_string());
        };
        let song_id = entry.song_id.clone();

        self.status.state = PlaybackState::Loading;
        self.status.loading_reason = Some(PlaybackLoadingReason::Buffering);
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id.clone());
        self.status.current_position_ms = requested_position_ms;
        self.status.error = None;
        self.report_status();

        let load_result =
            self.load_stream_for_song(context, &entry, requested_position_ms, autoplay);

        if let Err(error) = load_result {
            self.status.state = PlaybackState::Error;
            self.status.error = Some(error.clone());
            self.status.loading_reason = None;
            self.report_status();
            return Err(error);
        }

        self.status.state = if autoplay {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        };
        self.status.current_position_ms = requested_position_ms;
        self.status.current_song_id = Some(song_id);
        self.status.error = None;
        self.status.loading_reason = None;
        self.current_source_absolute_start_position_ms = Some(requested_position_ms);

        Ok(())
    }

    fn reload_track_at_index_exact(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        index: u32,
        requested_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String> {
        let Some(entry) = current_queue_entry(&self.status.queue, Some(index)).cloned() else {
            return Err("Current playback entry does not exist.".to_string());
        };
        let song_id = entry.song_id.clone();
        let previous_status = self.status.clone();
        let previous_source_absolute_start_position_ms =
            self.current_source_absolute_start_position_ms;

        self.status.state = PlaybackState::Loading;
        self.status.loading_reason = Some(PlaybackLoadingReason::Seeking);
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id.clone());
        self.status.current_position_ms = requested_position_ms;
        self.status.error = None;
        self.report_status();

        if let Err(error) =
            self.load_stream_for_song(context, &entry, requested_position_ms, autoplay)
        {
            self.status = previous_status;
            self.current_source_absolute_start_position_ms =
                previous_source_absolute_start_position_ms;
            self.report_status();
            return Err(error);
        }
        self.status.state = if autoplay {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        };
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id);
        self.status.current_position_ms = requested_position_ms;
        self.status.error = None;
        self.status.loading_reason = None;
        self.current_source_absolute_start_position_ms = Some(requested_position_ms);

        Ok(())
    }

    fn load_stream_for_song(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        entry: &PlaybackQueueEntry,
        requested_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String> {
        let song_id = entry.song_id.as_str();
        let supports_offset = context.capability_matrix.transcode_offset
            && !is_probably_flac_path(entry.path.as_deref());
        let stream_offset_seconds =
            stream_seek_offset_seconds(requested_position_ms, supports_offset);
        let local_start_position_ms =
            backend_seek_position_ms(requested_position_ms, supports_offset);

        log::info!(
            "controller.load_stream_for_song: song_id={} path={:?} requested_position_ms={} supports_offset={} stream_offset_seconds={:?} local_start_position_ms={} autoplay={}",
            song_id,
            entry.path,
            requested_position_ms,
            supports_offset,
            stream_offset_seconds,
            local_start_position_ms,
            autoplay
        );

        if requested_position_ms == 0 {
            let raw_stream =
                build_stream_request(context.client, song_id, stream_offset_seconds, true)?;
            if let Err(raw_error) = self.backend.load(
                raw_stream,
                requested_position_ms,
                local_start_position_ms,
                autoplay,
            ) {
                log::warn!(
                    "controller.load_track_at_index: raw stream load failed for song_id={song_id}: {raw_error}"
                );
                let fallback_stream =
                    build_stream_request(context.client, song_id, stream_offset_seconds, false)?;
                if let Err(fallback_error) = self.backend.load(
                    fallback_stream,
                    requested_position_ms,
                    local_start_position_ms,
                    autoplay,
                ) {
                    let message = format!(
                        "Failed to load playback stream. raw stream failed: {raw_error}; fallback stream failed: {fallback_error}"
                    );
                    log::error!(
                        "controller.load_track_at_index: fallback stream load failed for song_id={song_id}: {fallback_error}"
                    );
                    return Err(message);
                }
            }

            return Ok(());
        }

        let standard_stream =
            build_stream_request(context.client, song_id, stream_offset_seconds, false)?;
        self.backend
            .load(
                standard_stream,
                requested_position_ms,
                local_start_position_ms,
                autoplay,
            )
            .map_err(|error| format!("Failed to load playback stream: {error}"))
    }

    fn sync_current_position_from_backend(&mut self) -> Result<(), String> {
        if !matches!(
            self.status.state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            return Ok(());
        }

        let current_position_ms = self
            .backend
            .current_position_ms()
            .map_err(|error| format!("Failed to sync the backend playback position: {error}"))?;
        self.status.current_position_ms = current_position_ms;
        self.status.error = None;
        self.status.loading_reason = None;
        Ok(())
    }

    fn sync_queue_state(&mut self) {
        let _ = self.queue_sync.sync_queue(&self.status);
    }

    fn report_status(&mut self) {
        let _ = self.reporter.report_state(&self.status);
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

fn current_queue_entry(
    entries: &[PlaybackQueueEntry],
    current_index: Option<u32>,
) -> Option<&PlaybackQueueEntry> {
    let index = usize::try_from(current_index?).ok()?;
    entries.get(index)
}

fn is_probably_flac_path(path: Option<&str>) -> bool {
    path.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    })
    .and_then(|value| {
        std::path::Path::new(value)
            .extension()
            .and_then(|ext| ext.to_str())
    })
    .is_some_and(|extension| extension.eq_ignore_ascii_case("flac"))
}

fn stream_seek_offset_seconds(position_ms: u32, supports_offset: bool) -> Option<u32> {
    if supports_offset {
        position_ms_to_offset_seconds(position_ms)
    } else {
        None
    }
}

fn backend_seek_position_ms(position_ms: u32, supports_offset: bool) -> u32 {
    if supports_offset {
        position_ms % 1000
    } else {
        position_ms
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
        backend_seek_position_ms, current_song_id, is_probably_flac_path,
        position_ms_to_offset_seconds, stream_seek_offset_seconds, PlaybackController,
        PlaybackRuntimeContext, PlaybackStatus,
    };
    use crate::{
        models::{
            CapabilityMatrix, PlaybackLoadingReason, PlaybackQueueEntry, PlaybackSetQueueRequest,
            PlaybackState,
        },
        playback::{
            backend_shims::PlaybackBackend,
            queue_sync::{NoopQueueSyncGateway, QueueSyncGateway},
            reporting::PlaybackReporter,
        },
    };

    #[derive(Debug, Default)]
    struct MockBackendState {
        load_calls: Vec<String>,
        load_absolute_start_positions_ms: Vec<u32>,
        load_local_start_positions_ms: Vec<u32>,
        seek_calls_ms: Vec<u32>,
        current_position_ms: u32,
        current_position_calls: usize,
        current_source_absolute_start_position_ms: Option<u32>,
        fail_first_load: bool,
        always_fail_load: bool,
        fail_standard_load: bool,
        fail_seek: bool,
        has_failed_first_load: bool,
        pause_calls: usize,
        stop_calls: usize,
    }

    #[derive(Debug, Default)]
    struct MockReporterState {
        reported_states: Vec<PlaybackStatus>,
    }

    struct MockBackend {
        state: Arc<Mutex<MockBackendState>>,
    }

    struct MockReporter {
        state: Arc<Mutex<MockReporterState>>,
    }

    impl PlaybackBackend for MockBackend {
        fn load(
            &mut self,
            request: opensubsonic_client::PreparedBinaryRequest,
            absolute_start_position_ms: u32,
            local_start_position_ms: u32,
            _autoplay: bool,
        ) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.load_calls.push(request.url.to_string());
            state
                .load_absolute_start_positions_ms
                .push(absolute_start_position_ms);
            state
                .load_local_start_positions_ms
                .push(local_start_position_ms);
            if state.always_fail_load {
                return Err("stream rejected".to_string());
            }
            if state.fail_standard_load && !request.url.as_str().contains("format=raw") {
                return Err("standard stream rejected".to_string());
            }
            if state.fail_first_load && !state.has_failed_first_load {
                state.has_failed_first_load = true;
                return Err("raw stream rejected".to_string());
            }

            state.current_position_ms = absolute_start_position_ms;
            state.current_source_absolute_start_position_ms = Some(absolute_start_position_ms);

            Ok(())
        }

        fn seek(&mut self, position_ms: u32) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.seek_calls_ms.push(position_ms);
            if state.fail_seek {
                return Err("sink rejected seek".to_string());
            }
            state.current_position_ms = position_ms;
            Ok(())
        }

        fn current_position_ms(&self) -> Result<u32, String> {
            let mut state = self.state.lock().unwrap();
            state.current_position_calls += 1;
            Ok(state.current_position_ms)
        }

        fn pause(&mut self) -> Result<(), String> {
            self.state.lock().unwrap().pause_calls += 1;
            Ok(())
        }

        fn stop(&mut self) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.stop_calls += 1;
            state.current_position_ms = 0;
            state.current_source_absolute_start_position_ms = None;
            Ok(())
        }
    }

    impl PlaybackReporter for MockReporter {
        fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
            self.state
                .lock()
                .unwrap()
                .reported_states
                .push(status.clone());
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
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        controller_with_mock_backend_config(fail_first_load, false, false)
    }

    fn controller_with_mock_backend_config(
        fail_first_load: bool,
        always_fail_load: bool,
        fail_seek: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        controller_with_mock_backend_full_config(
            fail_first_load,
            always_fail_load,
            false,
            fail_seek,
        )
    }

    fn controller_with_mock_backend_full_config(
        fail_first_load: bool,
        always_fail_load: bool,
        fail_standard_load: bool,
        fail_seek: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState {
            fail_first_load,
            always_fail_load,
            fail_standard_load,
            fail_seek,
            ..MockBackendState::default()
        }));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state.clone(),
        });
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let controller = PlaybackController::new(backend, reporter, queue_sync);
        (controller, state, reporter_state)
    }

    fn queue_entries(song_ids: &[&str]) -> Vec<PlaybackQueueEntry> {
        song_ids
            .iter()
            .map(|song_id| PlaybackQueueEntry {
                song_id: (*song_id).to_string(),
                title: (*song_id).to_string(),
                path: None,
            })
            .collect()
    }

    fn queue_entries_with_paths(entries: &[(&str, Option<&str>)]) -> Vec<PlaybackQueueEntry> {
        entries
            .iter()
            .map(|(song_id, path)| PlaybackQueueEntry {
                song_id: (*song_id).to_string(),
                title: (*song_id).to_string(),
                path: path.map(str::to_string),
            })
            .collect()
    }

    #[test]
    fn set_queue_rejects_out_of_range_current_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);

        let result = controller.set_queue(PlaybackSetQueueRequest {
            entries: queue_entries(&["song-a"]),
            current_index: Some(1),
        });

        assert!(result.is_err());
    }

    #[test]
    fn duplicate_song_ids_are_distinguished_by_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
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
        let (mut controller, _, _) = controller_with_mock_backend(false);
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
        let (mut controller, _, _) = controller_with_mock_backend(false);
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
    fn synced_state_reads_backend_current_position() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
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

        backend_state.lock().unwrap().current_position_ms = 12_345;

        let synced = controller.synced_state().unwrap();
        assert_eq!(synced.current_position_ms, 12_345);
        assert_eq!(backend_state.lock().unwrap().current_position_calls, 1);
    }

    #[test]
    fn pause_captures_backend_position_before_transition() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
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
        backend_state.lock().unwrap().current_position_ms = 2_500;

        let paused = controller.pause().unwrap();
        assert_eq!(paused.state, PlaybackState::Paused);
        assert_eq!(paused.current_position_ms, 2_500);
    }

    #[test]
    fn on_track_finished_does_not_auto_advance_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
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
        let (mut controller, backend_state, _) = controller_with_mock_backend(true);
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
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
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
        assert_eq!(seeked.current_position_ms, 5_000);
        controller.play(&runtime_context).unwrap();

        let backend_state = backend_state.lock().unwrap();
        let load_calls = backend_state.load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert_eq!(backend_state.load_absolute_start_positions_ms, vec![5_000]);
        assert_eq!(backend_state.load_local_start_positions_ms, vec![5_000]);
        assert!(!load_calls[0].contains("timeOffset="));
    }

    #[test]
    fn active_seek_uses_backend_seek_without_reloading_stream() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
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
        controller.play(&runtime_context).unwrap();

        let seeked = controller.seek(&runtime_context, 5_500).unwrap();
        assert_eq!(seeked.current_position_ms, 5_500);

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.seek_calls_ms, vec![5_500]);
    }

    #[test]
    fn active_seek_reloads_stream_when_backend_seek_fails_using_standard_stream() {
        let (mut controller, backend_state, _) =
            controller_with_mock_backend_config(false, false, true);
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

        let seeked = controller.seek(&runtime_context, 5_500).unwrap();
        assert_eq!(seeked.current_position_ms, 5_500);

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.seek_calls_ms, vec![5_500]);
        assert_eq!(backend_state.load_calls.len(), 2);
        assert_eq!(
            backend_state.load_absolute_start_positions_ms,
            vec![0, 5_500]
        );
        assert_eq!(backend_state.load_local_start_positions_ms, vec![0, 500]);
        assert!(backend_state.load_calls[1].contains("timeOffset=5"));
        assert!(!backend_state.load_calls[1].contains("format=raw"));
    }

    #[test]
    fn flac_active_seek_always_reloads_without_native_seek() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries_with_paths(&[("song-a", Some("music/song-a.FLAC"))]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        let seeked = controller.seek(&runtime_context, 5_500).unwrap();
        assert_eq!(seeked.current_position_ms, 5_500);

        let backend_state = backend_state.lock().unwrap();
        assert!(backend_state.seek_calls_ms.is_empty());
        assert_eq!(backend_state.load_calls.len(), 2);
        assert!(!backend_state.load_calls[1].contains("timeOffset="));
        assert_eq!(backend_state.load_local_start_positions_ms, vec![0, 5_500]);
    }

    #[test]
    fn active_seek_after_nonzero_reload_skips_native_seek_and_uses_standard_stream() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
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

        controller.seek(&runtime_context, 5_000).unwrap();
        controller.play(&runtime_context).unwrap();
        controller.seek(&runtime_context, 7_000).unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert!(backend_state.seek_calls_ms.is_empty());
        assert_eq!(backend_state.load_calls.len(), 2);
        assert!(!backend_state.load_calls[0].contains("format=raw"));
        assert!(!backend_state.load_calls[1].contains("format=raw"));
        assert_eq!(
            backend_state.load_absolute_start_positions_ms,
            vec![5_000, 7_000]
        );
    }

    #[test]
    fn failed_exact_seek_preserves_last_confirmed_position() {
        let (mut controller, backend_state, _) =
            controller_with_mock_backend_full_config(false, false, true, true);
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
        backend_state.lock().unwrap().current_position_ms = 1_750;

        let seek_result = controller.seek(&runtime_context, 5_500);
        assert!(seek_result.is_err());

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.seek_calls_ms, vec![5_500]);
        assert_eq!(backend_state.load_calls.len(), 2);
        assert!(!backend_state.load_calls[1].contains("format=raw"));
        drop(backend_state);

        let status = controller.state();
        assert_eq!(status.state, PlaybackState::Playing);
        assert_eq!(status.current_position_ms, 1_750);
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
    fn helper_detects_flac_paths_case_insensitively() {
        assert!(is_probably_flac_path(Some("music/song.flac")));
        assert!(is_probably_flac_path(Some("music/song.FLAC")));
        assert!(!is_probably_flac_path(Some("music/song.mp3")));
        assert!(!is_probably_flac_path(None));
    }

    #[test]
    fn helper_stream_seek_offset_seconds_applies_capability_gate() {
        assert_eq!(stream_seek_offset_seconds(7_000, true), Some(7));
        assert_eq!(stream_seek_offset_seconds(7_000, false), None);
    }

    #[test]
    fn helper_backend_seek_position_ms_preserves_local_remainder() {
        assert_eq!(backend_seek_position_ms(7_500, true), 500);
        assert_eq!(backend_seek_position_ms(7_500, false), 7_500);
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

    #[test]
    fn reporter_receives_discrete_state_snapshots_for_mutations() {
        let (mut controller, _, reporter_state) = controller_with_mock_backend(false);
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
        controller.pause().unwrap();
        controller.seek(&runtime_context, 8_000).unwrap();
        controller.next(&runtime_context).unwrap();
        controller.prev(&runtime_context).unwrap();
        controller.stop().unwrap();

        let reported_states = reporter_state.lock().unwrap().reported_states.clone();
        let states = reported_states
            .iter()
            .map(|status| status.state.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            states,
            vec![
                PlaybackState::Stopped,
                PlaybackState::Loading,
                PlaybackState::Playing,
                PlaybackState::Paused,
                PlaybackState::Paused,
                PlaybackState::Loading,
                PlaybackState::Paused,
                PlaybackState::Loading,
                PlaybackState::Paused,
                PlaybackState::Stopped,
            ]
        );
        assert_eq!(
            reported_states[1].loading_reason,
            Some(PlaybackLoadingReason::Buffering)
        );
        assert_eq!(
            reported_states[5].loading_reason,
            Some(PlaybackLoadingReason::Buffering)
        );
        assert_eq!(
            reported_states[7].loading_reason,
            Some(PlaybackLoadingReason::Buffering)
        );
        assert_eq!(reported_states[4].current_position_ms, 8_000);
        assert_eq!(reported_states[5].current_index, Some(1));
        assert_eq!(reported_states[8].current_index, Some(0));
    }

    #[test]
    fn flac_seek_reports_loading_reason_as_seeking() {
        let (mut controller, _, reporter_state) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries_with_paths(&[("song-a", Some("music/song-a.flac"))]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        controller.seek(&runtime_context, 5_500).unwrap();

        let reported_states = reporter_state.lock().unwrap().reported_states.clone();
        assert_eq!(
            reported_states[3].loading_reason,
            Some(PlaybackLoadingReason::Seeking)
        );
        assert_eq!(reported_states[4].loading_reason, None);
    }

    #[test]
    fn load_failures_are_reported_as_error_snapshots() {
        let (mut controller, _, reporter_state) =
            controller_with_mock_backend_config(false, true, false);
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

        let result = controller.play(&runtime_context);
        assert!(result.is_err());

        let reported_states = reporter_state.lock().unwrap().reported_states.clone();
        let states = reported_states
            .iter()
            .map(|status| status.state.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            states,
            vec![
                PlaybackState::Stopped,
                PlaybackState::Loading,
                PlaybackState::Error,
            ]
        );
        assert!(reported_states
            .last()
            .and_then(|status| status.error.as_deref())
            .is_some_and(|message| message.contains("Failed to load playback stream.")));
    }

    #[test]
    fn reset_clears_status_and_reports_idle() {
        let (mut controller, _, reporter_state) = controller_with_mock_backend(false);
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

        let status = controller.reset();
        assert_eq!(status, PlaybackStatus::empty());

        let reported_states = reporter_state.lock().unwrap().reported_states.clone();
        assert_eq!(
            reported_states.last().cloned(),
            Some(PlaybackStatus::empty())
        );
    }
}
