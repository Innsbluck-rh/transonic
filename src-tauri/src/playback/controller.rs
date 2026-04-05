use opensubsonic_client::{
    api::retrieval::{RetrievalApi, StreamRequest},
    ApiError, OpenSubsonicClient, PreparedBinaryRequest,
};

use crate::{
    cover_art_cache::CoverArtCache,
    models::{
        CapabilityMatrix, InterruptReason, PlaybackSetQueueRequest, PlaybackStatus, PlayingState,
        SongResponse,
    },
};

use super::{
    backend_shims::{PlaybackBackend, PlaybackBackendLoadRequest, PlaybackSeekAction},
    native_events::{NativePlaybackEventSource, PlaybackNativeEvent},
    queue_sync::QueueSyncGateway,
    reporting::PlaybackReporter,
};

pub struct PlaybackRuntimeContext<'a> {
    pub client: &'a OpenSubsonicClient,
    pub capability_matrix: &'a CapabilityMatrix,
    pub cover_art_cache: Option<&'a CoverArtCache>,
    pub profile_id: Option<&'a str>,
}

pub struct PlaybackController {
    backend: Box<dyn PlaybackBackend>,
    reporter: Box<dyn PlaybackReporter>,
    queue_sync: Box<dyn QueueSyncGateway>,
    native_events: Box<dyn NativePlaybackEventSource>,
    status: PlaybackStatus,
    interrupted_resume_state: Option<PlayingState>,
}

impl PlaybackController {
    pub fn new(
        backend: Box<dyn PlaybackBackend>,
        reporter: Box<dyn PlaybackReporter>,
        queue_sync: Box<dyn QueueSyncGateway>,
        native_events: Box<dyn NativePlaybackEventSource>,
    ) -> Self {
        Self {
            backend,
            reporter,
            queue_sync,
            native_events,
            status: PlaybackStatus::empty(),
            interrupted_resume_state: None,
        }
    }

    pub fn state(&self) -> PlaybackStatus {
        self.status.clone()
    }

    pub fn synced_state(&mut self) -> Result<PlaybackStatus, String> {
        self.process_native_events_inner(None, false)?;
        self.sync_current_position_from_backend()?;
        Ok(self.state())
    }

    #[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
    pub fn process_native_events(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<(), String> {
        let _ = self.process_native_events_inner(context, true)?;
        Ok(())
    }

    pub fn set_queue(
        &mut self,
        payload: PlaybackSetQueueRequest,
    ) -> Result<PlaybackStatus, String> {
        validate_queue_index(&payload.entries, payload.current_index)?;
        self.backend.stop()?;
        self.clear_native_events();

        let next_song_id = current_song_id(&payload.entries, payload.current_index);
        self.status.queue = payload.entries;
        self.status.current_index = payload.current_index;
        self.status.current_position_ms = 0;
        self.status.current_song_id = next_song_id;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.status.playing_state = if self.status.queue.is_empty() {
            PlayingState::Idle
        } else {
            PlayingState::Stopped
        };

        if let Some(entry) = current_queue_entry(&self.status.queue, self.status.current_index) {
            log::info!(
                "controller.set_queue: current_index={:?} song_id={} path={:?}",
                self.status.current_index,
                entry.id,
                entry.path
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
        self.process_native_events_inner(Some(context), false)?;
        self.report_status();
        Ok(self.state())
    }

    pub fn play_queue_index(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        index: u32,
    ) -> Result<PlaybackStatus, String> {
        if match usize::try_from(index).ok() {
            Some(idx) => idx >= self.status.queue.len(),
            None => true,
        } {
            return Err("Playback queue index is out of range.".to_string());
        }

        self.backend.stop()?;
        self.clear_native_events();
        self.status.current_index = Some(index);
        self.status.current_song_id = current_song_id(&self.status.queue, Some(index));
        self.status.current_position_ms = 0;
        self.status.playing_state = PlayingState::Stopped;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;

        self.play(context)
    }

    pub fn pause(&mut self) -> Result<PlaybackStatus, String> {
        self.sync_current_position_from_backend()?;
        self.backend.pause()?;
        self.status.playing_state = PlayingState::Paused;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.process_native_events_inner(None, false)?;
        self.report_status();
        Ok(self.state())
    }

    pub fn stop(&mut self) -> Result<PlaybackStatus, String> {
        self.backend.stop()?;
        self.clear_native_events();
        self.status.playing_state = PlayingState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.report_status();
        Ok(self.state())
    }

    pub fn seek(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        requested_position_ms: u32,
    ) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        let autoplay = self.should_autoplay();
        let entry = self
            .status
            .queue
            .get(
                usize::try_from(index)
                    .map_err(|_| "Current playback index is out of range.".to_string())?,
            )
            .cloned();

        log::info!(
            "controller.seek: song_id={:?} path={:?} state={:?} requested_position_ms={} current_position_ms={}",
            entry.as_ref().map(|queue_entry| queue_entry.id.as_str()),
            entry.as_ref().and_then(|queue_entry| queue_entry.path.as_deref()),
            self.status.playing_state,
            requested_position_ms,
            self.status.current_position_ms
        );

        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            self.sync_current_position_from_backend()?;

            match self.backend.seek(requested_position_ms)? {
                PlaybackSeekAction::Applied => {
                    log::info!(
                        "controller.seek: backend applied native seek at position_ms={requested_position_ms}"
                    );
                    self.status.error = None;
                    self.status.interrupt_reason = None;
                    self.status.pending_seek_position_ms = Some(requested_position_ms);
                }
                PlaybackSeekAction::ReloadRequired => {
                    log::info!(
                        "controller.seek: backend requested exact reload at position_ms={requested_position_ms}"
                    );
                    self.reload_track_at_index_exact(
                        context,
                        index,
                        requested_position_ms,
                        autoplay,
                    )?;
                }
            }
        } else {
            self.status.current_position_ms = requested_position_ms;
            self.status.error = None;
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.interrupted_resume_state = None;
        }

        self.process_native_events_inner(Some(context), false)?;
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

        let autoplay = self.should_autoplay();
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            self.load_track_at_index(context, next_index, 0, autoplay)?;
        } else {
            self.status.current_index = Some(next_index);
            self.status.current_song_id = current_song_id(&self.status.queue, Some(next_index));
            self.status.current_position_ms = 0;
            self.status.playing_state = PlayingState::Stopped;
            self.status.error = None;
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.interrupted_resume_state = None;
        }

        self.process_native_events_inner(Some(context), false)?;
        self.report_status();
        Ok(self.state())
    }

    pub fn prev(&mut self, context: &PlaybackRuntimeContext<'_>) -> Result<PlaybackStatus, String> {
        let current_index = self.ensure_current_index()?;
        if current_index == 0 {
            return Err("Already at the beginning of the playback queue.".to_string());
        }

        let prev_index = current_index - 1;
        let autoplay = self.should_autoplay();
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            self.load_track_at_index(context, prev_index, 0, autoplay)?;
        } else {
            self.status.current_index = Some(prev_index);
            self.status.current_song_id = current_song_id(&self.status.queue, Some(prev_index));
            self.status.current_position_ms = 0;
            self.status.playing_state = PlayingState::Stopped;
            self.status.error = None;
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.interrupted_resume_state = None;
        }

        self.process_native_events_inner(Some(context), false)?;
        self.report_status();
        Ok(self.state())
    }

    #[allow(dead_code)]
    pub fn on_track_finished(&mut self) -> PlaybackStatus {
        self.clear_native_events();
        self.status.playing_state = PlayingState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.report_status();
        self.state()
    }

    pub fn reset(&mut self) -> PlaybackStatus {
        if let Err(error) = self.backend.stop() {
            log::warn!("controller.reset: failed to stop backend: {error}");
        }

        self.clear_native_events();
        self.status = PlaybackStatus::empty();
        self.interrupted_resume_state = None;
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
        let song_id = entry.id.clone();

        self.clear_native_events();
        self.status.playing_state = PlayingState::Interrupted;
        self.status.interrupt_reason = Some(InterruptReason::InitialLoad);
        self.status.pending_seek_position_ms = None;
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id.clone());
        self.status.current_position_ms = requested_position_ms;
        self.status.error = None;
        self.interrupted_resume_state = None;
        self.report_status();

        let load_result =
            self.load_stream_for_song(context, &entry, requested_position_ms, autoplay);

        if let Err(error) = load_result {
            self.status.playing_state = PlayingState::Error;
            self.status.error = Some(error.clone());
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.report_status();
            return Err(error);
        }

        self.clear_native_events();
        self.status.playing_state = if autoplay {
            PlayingState::Playing
        } else {
            PlayingState::Paused
        };
        self.status.current_position_ms = requested_position_ms;
        self.status.current_song_id = Some(song_id);
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;

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
        let song_id = entry.id.clone();
        let previous_status = self.status.clone();
        let previous_resume_state = self.interrupted_resume_state.clone();

        self.clear_native_events();
        self.status.playing_state = PlayingState::Interrupted;
        self.status.interrupt_reason = Some(InterruptReason::FullReload);
        self.status.pending_seek_position_ms = Some(requested_position_ms);
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id.clone());
        self.status.error = None;
        self.interrupted_resume_state = None;
        self.report_status();

        if let Err(error) =
            self.load_stream_for_song(context, &entry, requested_position_ms, autoplay)
        {
            self.status = previous_status;
            self.interrupted_resume_state = previous_resume_state;
            self.report_status();
            return Err(error);
        }
        self.clear_native_events();
        self.status.playing_state = if autoplay {
            PlayingState::Playing
        } else {
            PlayingState::Paused
        };
        self.status.current_index = Some(index);
        self.status.current_song_id = Some(song_id);
        self.status.current_position_ms = requested_position_ms;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;

        Ok(())
    }

    fn load_stream_for_song(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        entry: &SongResponse,
        requested_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String> {
        let song_id = entry.id.as_str();
        let load_strategy = self.backend.plan_load(
            requested_position_ms,
            context.capability_matrix.transcode_offset,
        );
        let stream_offset_seconds = load_strategy.stream_offset_seconds;
        let local_start_position_ms = load_strategy.local_start_position_ms;

        log::info!(
            "controller.load_stream_for_song: song_id={} path={:?} requested_position_ms={} server_offset_capability={} stream_offset_seconds={:?} local_start_position_ms={} autoplay={}",
            song_id,
            entry.path,
            requested_position_ms,
            context.capability_matrix.transcode_offset,
            stream_offset_seconds,
            local_start_position_ms,
            autoplay
        );

        let artwork_path = build_cover_art_path(
            context.cover_art_cache,
            context.client,
            context.profile_id,
            entry.cover_art_id.as_deref(),
        )?;

        if requested_position_ms == 0 {
            let raw_stream =
                build_stream_request(context.client, song_id, stream_offset_seconds, true)?;
            if let Err(raw_error) = self.backend.load(PlaybackBackendLoadRequest {
                request: raw_stream,
                media_id: entry.id.clone(),
                title: entry.title.clone(),
                artist: entry.artist.clone(),
                album: entry.album.clone(),
                artwork_path: artwork_path.clone(),
                absolute_start_position_ms: requested_position_ms,
                local_start_position_ms,
                autoplay,
            }) {
                log::warn!(
                    "controller.load_track_at_index: raw stream load failed for song_id={song_id}: {raw_error}"
                );
                let fallback_stream =
                    build_stream_request(context.client, song_id, stream_offset_seconds, false)?;
                if let Err(fallback_error) = self.backend.load(PlaybackBackendLoadRequest {
                    request: fallback_stream,
                    media_id: entry.id.clone(),
                    title: entry.title.clone(),
                    artist: entry.artist.clone(),
                    album: entry.album.clone(),
                    artwork_path,
                    absolute_start_position_ms: requested_position_ms,
                    local_start_position_ms,
                    autoplay,
                }) {
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
            .load(PlaybackBackendLoadRequest {
                request: standard_stream,
                media_id: entry.id.clone(),
                title: entry.title.clone(),
                artist: entry.artist.clone(),
                album: entry.album.clone(),
                artwork_path,
                absolute_start_position_ms: requested_position_ms,
                local_start_position_ms,
                autoplay,
            })
            .map_err(|error| format!("Failed to load playback stream: {error}"))
    }

    fn sync_current_position_from_backend(&mut self) -> Result<(), String> {
        if !matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            return Ok(());
        }

        let current_position_ms = self
            .backend
            .current_position_ms()
            .map_err(|error| format!("Failed to sync the backend playback position: {error}"))?;
        self.status.current_position_ms = current_position_ms;
        self.status.error = None;
        Ok(())
    }

    fn process_native_events_inner(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
        report_changes: bool,
    ) -> Result<bool, String> {
        let events = self.native_events.drain_events();
        if events.is_empty() {
            return Ok(false);
        }

        let mut changed = false;
        for event in events {
            changed |= self.apply_native_event(event, context)?;
        }

        if changed && report_changes {
            self.report_status();
        }

        Ok(changed)
    }

    fn apply_native_event(
        &mut self,
        event: PlaybackNativeEvent,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<bool, String> {
        match event {
            PlaybackNativeEvent::Buffering { position_ms } => {
                if !matches!(self.status.playing_state, PlayingState::Interrupted) {
                    self.interrupted_resume_state = Some(self.status.playing_state.clone());
                }
                self.status.playing_state = PlayingState::Interrupted;
                self.status.interrupt_reason =
                    Some(if self.status.pending_seek_position_ms.is_some() {
                        InterruptReason::Seeking
                    } else {
                        InterruptReason::StreamBufferingStall
                    });
                self.status.current_position_ms = position_ms;
                self.status.error = None;
                Ok(true)
            }
            PlaybackNativeEvent::Ready { position_ms } => {
                self.status.current_position_ms = position_ms;
                self.status.error = None;
                self.confirm_pending_seek();
                self.status.interrupt_reason = None;
                if matches!(self.status.playing_state, PlayingState::Interrupted) {
                    self.status.playing_state = self
                        .interrupted_resume_state
                        .clone()
                        .unwrap_or(PlayingState::Paused);
                }
                self.interrupted_resume_state = None;
                Ok(true)
            }
            PlaybackNativeEvent::Playing { position_ms } => {
                self.status.playing_state = PlayingState::Playing;
                self.status.interrupt_reason = None;
                self.status.current_position_ms = position_ms;
                self.status.error = None;
                self.confirm_pending_seek();
                self.interrupted_resume_state = None;
                Ok(true)
            }
            PlaybackNativeEvent::Paused { position_ms } => {
                self.status.playing_state = PlayingState::Paused;
                self.status.interrupt_reason = None;
                self.status.current_position_ms = position_ms;
                self.status.error = None;
                self.confirm_pending_seek();
                self.interrupted_resume_state = None;
                Ok(true)
            }
            PlaybackNativeEvent::SeekProcessed { position_ms } => {
                self.status.current_position_ms = position_ms;
                self.status.error = None;
                self.confirm_pending_seek();
                Ok(true)
            }
            PlaybackNativeEvent::Error { message } => {
                self.status.playing_state = PlayingState::Error;
                self.status.interrupt_reason = None;
                self.status.pending_seek_position_ms = None;
                self.status.error = Some(message);
                self.interrupted_resume_state = None;
                Ok(true)
            }
            PlaybackNativeEvent::Ended => self.handle_track_ended(context),
        }
    }

    fn handle_track_ended(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<bool, String> {
        let Some(current_index) = self.status.current_index else {
            return Ok(false);
        };
        let Some(next_index) = current_index.checked_add(1) else {
            self.transition_to_stopped_end_of_queue();
            return Ok(true);
        };
        let has_next = usize::try_from(next_index)
            .ok()
            .is_some_and(|index| index < self.status.queue.len());
        if !has_next {
            self.transition_to_stopped_end_of_queue();
            return Ok(true);
        }

        let Some(runtime_context) = context else {
            self.transition_to_stopped_end_of_queue();
            return Ok(true);
        };
        self.load_track_at_index(runtime_context, next_index, 0, true)?;
        self.report_status();
        Ok(false)
    }

    fn transition_to_stopped_end_of_queue(&mut self) {
        self.status.playing_state = PlayingState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
    }

    fn clear_native_events(&mut self) {
        let _ = self.native_events.drain_events();
    }

    fn should_autoplay(&self) -> bool {
        matches!(self.status.playing_state, PlayingState::Playing)
            || matches!(self.interrupted_resume_state, Some(PlayingState::Playing))
    }

    fn confirm_pending_seek(&mut self) {
        self.status.pending_seek_position_ms = None;
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

fn build_cover_art_path(
    cover_art_cache: Option<&CoverArtCache>,
    client: &OpenSubsonicClient,
    profile_id: Option<&str>,
    cover_art_id: Option<&str>,
) -> Result<Option<String>, String> {
    let (Some(cover_art_cache), Some(profile_id), Some(cover_art_id)) =
        (cover_art_cache, profile_id, cover_art_id)
    else {
        return Ok(None);
    };

    let cover_art_cache = cover_art_cache.clone();
    let client = client.clone();
    let profile_id = profile_id.to_string();
    let cover_art_id = cover_art_id.to_string();
    let handle = std::thread::Builder::new()
        .name("cover-art-playback-resolve".to_string())
        .spawn(move || {
            cover_art_cache.resolve_cover_art(&client, &profile_id, &cover_art_id, Some(512))
        })
        .map_err(|error| format!("Failed to start the cover art resolver thread: {error}"))?;

    handle
        .join()
        .map_err(|_| "The cover art resolver thread panicked.".to_string())?
        .map(|path| Some(path.to_string_lossy().to_string()))
}

fn validate_queue_index(
    entries: &[SongResponse],
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

fn current_song_id(entries: &[SongResponse], current_index: Option<u32>) -> Option<String> {
    let index = usize::try_from(current_index?).ok()?;
    entries.get(index).map(|entry| entry.id.clone())
}

fn current_queue_entry(
    entries: &[SongResponse],
    current_index: Option<u32>,
) -> Option<&SongResponse> {
    let index = usize::try_from(current_index?).ok()?;
    entries.get(index)
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

    use super::{current_song_id, PlaybackController, PlaybackRuntimeContext, PlaybackStatus};
    use crate::{
        models::{
            CapabilityMatrix, InterruptReason, PlaybackSetQueueRequest, PlayingState, SongResponse,
        },
        playback::{
            backend_shims::{
                PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy,
                PlaybackSeekAction,
            },
            native_events::{
                NativePlaybackEventSource, NoopNativePlaybackEventSource, PlaybackNativeEvent,
            },
            queue_sync::{NoopQueueSyncGateway, QueueSyncGateway},
            reporting::PlaybackReporter,
        },
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    enum MockSeekBehavior {
        #[default]
        Apply,
        ReloadRequired,
        Fail,
    }

    #[derive(Debug, Default)]
    struct MockBackendState {
        load_calls: Vec<String>,
        load_absolute_start_positions_ms: Vec<u32>,
        load_local_start_positions_ms: Vec<u32>,
        seek_calls_ms: Vec<u32>,
        current_position_ms: u32,
        current_position_calls: usize,
        fail_first_load: bool,
        always_fail_load: bool,
        fail_standard_load: bool,
        seek_behavior: MockSeekBehavior,
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

    #[derive(Default)]
    struct MockNativeEventSource {
        events: Arc<Mutex<Vec<PlaybackNativeEvent>>>,
    }

    impl PlaybackBackend for MockBackend {
        fn plan_load(
            &self,
            requested_position_ms: u32,
            supports_stream_offset: bool,
        ) -> PlaybackLoadStrategy {
            PlaybackLoadStrategy::split_by_stream_offset(
                requested_position_ms,
                supports_stream_offset,
            )
        }

        fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.load_calls.push(request.request.url.to_string());
            state
                .load_absolute_start_positions_ms
                .push(request.absolute_start_position_ms);
            state
                .load_local_start_positions_ms
                .push(request.local_start_position_ms);
            if state.always_fail_load {
                return Err("stream rejected".to_string());
            }
            if state.fail_standard_load && !request.request.url.as_str().contains("format=raw") {
                return Err("standard stream rejected".to_string());
            }
            if state.fail_first_load && !state.has_failed_first_load {
                state.has_failed_first_load = true;
                return Err("raw stream rejected".to_string());
            }

            state.current_position_ms = request.absolute_start_position_ms;

            Ok(())
        }

        fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String> {
            let mut state = self.state.lock().unwrap();
            state.seek_calls_ms.push(position_ms);
            match state.seek_behavior {
                MockSeekBehavior::Apply => {
                    state.current_position_ms = position_ms;
                    Ok(PlaybackSeekAction::Applied)
                }
                MockSeekBehavior::ReloadRequired => Ok(PlaybackSeekAction::ReloadRequired),
                MockSeekBehavior::Fail => Err("sink rejected seek".to_string()),
            }
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

    impl NativePlaybackEventSource for MockNativeEventSource {
        fn drain_events(&mut self) -> Vec<PlaybackNativeEvent> {
            let mut events = self.events.lock().unwrap();
            std::mem::take(&mut *events)
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
            seek_behavior: if fail_seek {
                MockSeekBehavior::Fail
            } else {
                MockSeekBehavior::Apply
            },
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
        let controller = PlaybackController::new(
            backend,
            reporter,
            queue_sync,
            Box::new(NoopNativePlaybackEventSource),
        );
        (controller, state, reporter_state)
    }

    fn controller_with_mock_backend_and_native_events(
        fail_first_load: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
        Arc<Mutex<Vec<PlaybackNativeEvent>>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState {
            fail_first_load,
            ..MockBackendState::default()
        }));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let native_events = Arc::new(Mutex::new(Vec::new()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state.clone(),
        });
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let controller = PlaybackController::new(
            backend,
            reporter,
            queue_sync,
            Box::new(MockNativeEventSource {
                events: native_events.clone(),
            }),
        );
        (controller, state, reporter_state, native_events)
    }

    fn controller_with_mock_backend_seek_behavior(
        seek_behavior: MockSeekBehavior,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let (controller, state, reporter_state) =
            controller_with_mock_backend_full_config(false, false, false, false);
        state.lock().unwrap().seek_behavior = seek_behavior;
        (controller, state, reporter_state)
    }

    fn queue_entries(song_ids: &[&str]) -> Vec<SongResponse> {
        song_ids
            .iter()
            .map(|song_id| SongResponse {
                id: (*song_id).to_string(),
                parent_id: None,
                path: None,
                title: (*song_id).to_string(),
                album: None,
                album_id: None,
                artist: None,
                artist_id: None,
                cover_art_id: None,
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
            cover_art_cache: None,
            profile_id: None,
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
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();

        assert_eq!(
            controller.play(&runtime_context).unwrap().playing_state,
            PlayingState::Playing
        );
        assert_eq!(
            controller.pause().unwrap().playing_state,
            PlayingState::Paused
        );
        let seeked = controller.seek(&runtime_context, 8_000).unwrap();
        assert_eq!(seeked.playing_state, PlayingState::Paused);
        assert_eq!(seeked.pending_seek_position_ms, Some(8_000));
        assert_eq!(
            controller.stop().unwrap().playing_state,
            PlayingState::Stopped
        );
    }

    #[test]
    fn synced_state_reads_backend_current_position() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
            cover_art_cache: None,
            profile_id: None,
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
        assert_eq!(paused.playing_state, PlayingState::Paused);
        assert_eq!(paused.current_position_ms, 2_500);
    }

    #[test]
    fn on_track_finished_does_not_auto_advance_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a", "song-b"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        let status = controller.on_track_finished();
        assert_eq!(status.playing_state, PlayingState::Stopped);
        assert_eq!(status.current_index, Some(0));
    }

    #[test]
    fn native_paused_event_is_applied_before_synced_state_returns() {
        let (mut controller, backend_state, _, native_events) =
            controller_with_mock_backend_and_native_events(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();
        backend_state.lock().unwrap().current_position_ms = 4_321;
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Paused { position_ms: 4_321 });

        let synced = controller.synced_state().unwrap();

        assert_eq!(synced.playing_state, PlayingState::Paused);
        assert_eq!(synced.current_position_ms, 4_321);
    }

    #[test]
    fn ended_native_event_auto_advances_to_next_track() {
        let (mut controller, backend_state, _, native_events) =
            controller_with_mock_backend_and_native_events(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a", "song-b"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Ended);

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-b"));
        assert_eq!(backend_state.lock().unwrap().load_calls.len(), 2);
    }

    #[test]
    fn ended_native_event_stops_at_end_of_queue() {
        let (mut controller, _, _, native_events) =
            controller_with_mock_backend_and_native_events(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Ended);

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Stopped);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_position_ms, 0);
    }

    #[test]
    fn raw_stream_falls_back_to_standard_stream_when_backend_rejects_raw() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(true);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
            cover_art_cache: None,
            profile_id: None,
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
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        let seeked = controller.seek(&runtime_context, 5_500).unwrap();
        assert_eq!(seeked.current_position_ms, 0);
        assert_eq!(seeked.pending_seek_position_ms, Some(5_500));

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.seek_calls_ms, vec![5_500]);
    }

    #[test]
    fn active_seek_reloads_stream_when_backend_requires_reload_using_standard_stream() {
        let (mut controller, backend_state, _) =
            controller_with_mock_backend_seek_behavior(MockSeekBehavior::ReloadRequired);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
        assert_eq!(seeked.pending_seek_position_ms, None);

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
    fn repeated_reload_required_seeks_continue_using_standard_stream() {
        let (mut controller, backend_state, _) =
            controller_with_mock_backend_seek_behavior(MockSeekBehavior::ReloadRequired);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
        assert_eq!(backend_state.load_calls.len(), 2);
        assert_eq!(backend_state.seek_calls_ms, vec![7_000]);
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
            controller_with_mock_backend_seek_behavior(MockSeekBehavior::ReloadRequired);
        backend_state.lock().unwrap().fail_standard_load = true;
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
        assert_eq!(status.playing_state, PlayingState::Playing);
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
    fn play_queue_index_rejects_out_of_range_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(false);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };

        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();

        let result = controller.play_queue_index(&runtime_context, 1);

        assert!(result.is_err());
    }

    #[test]
    fn play_queue_index_starts_selected_track_from_zero() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(false);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };

        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a", "song-b"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.seek(&runtime_context, 5_000).unwrap();

        let status = controller.play_queue_index(&runtime_context, 1).unwrap();

        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-b"));
        assert_eq!(status.current_position_ms, 0);
        assert_eq!(status.playing_state, PlayingState::Playing);
    }

    #[test]
    fn load_strategy_split_by_stream_offset_uses_server_seconds_and_local_remainder() {
        assert_eq!(
            PlaybackLoadStrategy::split_by_stream_offset(7_500, true),
            PlaybackLoadStrategy {
                stream_offset_seconds: Some(7),
                local_start_position_ms: 500,
            }
        );
        assert_eq!(
            PlaybackLoadStrategy::split_by_stream_offset(7_500, false),
            PlaybackLoadStrategy {
                stream_offset_seconds: None,
                local_start_position_ms: 7_500,
            }
        );
        assert_eq!(
            PlaybackLoadStrategy::split_by_stream_offset(0, true),
            PlaybackLoadStrategy {
                stream_offset_seconds: None,
                local_start_position_ms: 0,
            }
        );
    }

    #[test]
    fn default_status_is_idle() {
        assert_eq!(PlaybackStatus::empty().playing_state, PlayingState::Idle);
    }

    #[test]
    fn reporter_receives_discrete_state_snapshots_for_mutations() {
        let (mut controller, _, reporter_state) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
            .map(|status| status.playing_state.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            states,
            vec![
                PlayingState::Stopped,
                PlayingState::Interrupted,
                PlayingState::Playing,
                PlayingState::Paused,
                PlayingState::Paused,
                PlayingState::Interrupted,
                PlayingState::Paused,
                PlayingState::Interrupted,
                PlayingState::Paused,
                PlayingState::Stopped,
            ]
        );
        assert_eq!(
            reported_states[1].interrupt_reason,
            Some(InterruptReason::InitialLoad)
        );
        assert_eq!(reported_states[4].pending_seek_position_ms, Some(8_000));
        assert_eq!(
            reported_states[5].interrupt_reason,
            Some(InterruptReason::InitialLoad)
        );
        assert_eq!(
            reported_states[7].interrupt_reason,
            Some(InterruptReason::InitialLoad)
        );
        assert_eq!(reported_states[5].current_index, Some(1));
        assert_eq!(reported_states[8].current_index, Some(0));
    }

    #[test]
    fn reload_required_seek_reports_full_reload_interruption() {
        let (mut controller, _, reporter_state) =
            controller_with_mock_backend_seek_behavior(MockSeekBehavior::ReloadRequired);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        controller.seek(&runtime_context, 5_500).unwrap();

        let reported_states = reporter_state.lock().unwrap().reported_states.clone();
        assert_eq!(
            reported_states[3].interrupt_reason,
            Some(InterruptReason::FullReload)
        );
        assert_eq!(reported_states[3].pending_seek_position_ms, Some(5_500));
        assert_eq!(reported_states[4].interrupt_reason, None);
        assert_eq!(reported_states[4].pending_seek_position_ms, None);
    }

    #[test]
    fn native_seek_is_cleared_by_seek_processed_event() {
        let (mut controller, _, _, native_events) =
            controller_with_mock_backend_and_native_events(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller.seek(&runtime_context, 5_500).unwrap();

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::SeekProcessed { position_ms: 5_500 });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_position_ms, 5_500);
        assert_eq!(status.pending_seek_position_ms, None);
    }

    #[test]
    fn runtime_buffering_reports_stream_buffering_stall() {
        let (mut controller, _, _, native_events) =
            controller_with_mock_backend_and_native_events(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Buffering { position_ms: 1_234 });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Interrupted);
        assert_eq!(
            status.interrupt_reason,
            Some(InterruptReason::StreamBufferingStall)
        );
        assert_eq!(status.pending_seek_position_ms, None);
    }

    #[test]
    fn seek_buffering_reports_seeking_interruption_until_ready() {
        let (mut controller, _, _, native_events) =
            controller_with_mock_backend_and_native_events(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller.seek(&runtime_context, 5_500).unwrap();

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Buffering { position_ms: 4_000 });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let interrupted = controller.state();
        assert_eq!(interrupted.playing_state, PlayingState::Interrupted);
        assert_eq!(interrupted.interrupt_reason, Some(InterruptReason::Seeking));
        assert_eq!(interrupted.pending_seek_position_ms, Some(5_500));

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Ready { position_ms: 5_500 });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let resumed = controller.state();
        assert_eq!(resumed.playing_state, PlayingState::Playing);
        assert_eq!(resumed.current_position_ms, 5_500);
        assert_eq!(resumed.pending_seek_position_ms, None);
    }

    #[test]
    fn set_queue_clears_pending_seek_position() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-a"]),
                current_index: Some(0),
            })
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller.seek(&runtime_context, 5_500).unwrap();

        let status = controller
            .set_queue(PlaybackSetQueueRequest {
                entries: queue_entries(&["song-b"]),
                current_index: Some(0),
            })
            .unwrap();

        assert_eq!(status.playing_state, PlayingState::Stopped);
        assert_eq!(status.pending_seek_position_ms, None);
    }

    #[test]
    fn load_failures_are_reported_as_error_snapshots() {
        let (mut controller, _, reporter_state) =
            controller_with_mock_backend_config(false, true, false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
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
            .map(|status| status.playing_state.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            states,
            vec![
                PlayingState::Stopped,
                PlayingState::Interrupted,
                PlayingState::Error,
            ]
        );
        assert_eq!(
            reported_states[1].interrupt_reason,
            Some(InterruptReason::InitialLoad)
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
            cover_art_cache: None,
            profile_id: None,
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
