use opensubsonic_client::{
    api::retrieval::{RetrievalApi, StreamRequest},
    ApiError, MediaType, OpenSubsonicClient, PreparedBinaryRequest,
};

use crate::{
    cover_art_cache::CoverArtCache,
    models::{
        CapabilityMatrix, GaplessState, GaplessStatus, InterruptReason, PlaybackCapabilities,
        PlaybackStatus, PlayingState, SongResponse,
    },
    playback_state::{PlaybackStateFile, PlaybackStatePersister},
};

use super::{
    backend_shims::{PlaybackBackend, PlaybackBackendLoadRequest, PlaybackSeekAction},
    native_events::{NativePlaybackEventSource, PlaybackNativeEvent},
    queue_sync::QueueSyncGateway,
    reporting::PlaybackReporter,
    server_reporting::{
        PlaybackServerReporter, ServerPlaybackReportingContext, ServerPlaybackState,
        ServerPlaybackTrack,
    },
};

pub struct PlaybackRuntimeContext<'a> {
    pub client: &'a OpenSubsonicClient,
    pub capability_matrix: &'a CapabilityMatrix,
    pub cover_art_cache: Option<&'a CoverArtCache>,
    pub profile_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GaplessTrackTarget {
    queue_index: u32,
    song_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreparedGaplessTrackState {
    Preparing {
        target: GaplessTrackTarget,
        generation: u64,
    },
    Ready {
        target: GaplessTrackTarget,
        generation: u64,
    },
    Failed {
        target: GaplessTrackTarget,
        generation: Option<u64>,
    },
}

impl PreparedGaplessTrackState {
    fn target(&self) -> &GaplessTrackTarget {
        match self {
            Self::Preparing { target, .. }
            | Self::Ready { target, .. }
            | Self::Failed { target, .. } => target,
        }
    }

    fn generation(&self) -> Option<u64> {
        match self {
            Self::Preparing { generation, .. } | Self::Ready { generation, .. } => {
                Some(*generation)
            }
            Self::Failed { generation, .. } => *generation,
        }
    }
}

pub struct PlaybackController {
    backend: Box<dyn PlaybackBackend>,
    reporter: Box<dyn PlaybackReporter>,
    server_reporter: Box<dyn PlaybackServerReporter>,
    queue_sync: Box<dyn QueueSyncGateway>,
    native_events: Box<dyn NativePlaybackEventSource>,
    persister: Box<dyn PlaybackStatePersister>,
    playback_capabilities: PlaybackCapabilities,
    gapless_playback_enabled: bool,
    prepared_next: Option<PreparedGaplessTrackState>,
    gapless_failure: Option<String>,
    status: PlaybackStatus,
    interrupted_resume_state: Option<PlayingState>,
    active_profile_id: Option<String>,
}

impl PlaybackController {
    pub fn new(
        backend: Box<dyn PlaybackBackend>,
        reporter: Box<dyn PlaybackReporter>,
        server_reporter: Box<dyn PlaybackServerReporter>,
        queue_sync: Box<dyn QueueSyncGateway>,
        native_events: Box<dyn NativePlaybackEventSource>,
        persister: Box<dyn PlaybackStatePersister>,
        gapless_playback_enabled: bool,
    ) -> Self {
        let playback_capabilities = backend.capabilities();
        Self {
            backend,
            reporter,
            server_reporter,
            queue_sync,
            native_events,
            persister,
            playback_capabilities,
            gapless_playback_enabled,
            prepared_next: None,
            gapless_failure: None,
            status: PlaybackStatus::empty(),
            interrupted_resume_state: None,
            active_profile_id: None,
        }
    }

    pub fn set_active_profile_id(&mut self, profile_id: Option<String>) {
        if self.active_profile_id != profile_id {
            self.clear_gapless_preparation();
            self.clear_server_reporting();
        }
        self.active_profile_id = profile_id;
    }

    pub fn clear_gapless_preparation(&mut self) {
        self.backend.clear_prepared();
        self.prepared_next = None;
        self.gapless_failure = None;
        self.update_gapless_status();
    }

    pub fn set_gapless_playback_enabled(
        &mut self,
        enabled: bool,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<(), String> {
        if self.gapless_playback_enabled == enabled {
            return Ok(());
        }
        self.gapless_playback_enabled = enabled;
        let result = self.sync_gapless_for_current_track(context);
        self.report_status();
        result
    }

    pub fn restore_state(
        &mut self,
        profile_id: String,
        queue: Vec<SongResponse>,
        current_index: Option<u32>,
        current_position_ms: u32,
    ) -> PlaybackStatus {
        if let Err(error) = validate_queue_index(&queue, current_index) {
            log::warn!(
                "controller.restore_state: invalid persisted index, starting fresh: {error}"
            );
            self.active_profile_id = Some(profile_id);
            return self.state();
        }

        let next_song_id = current_song_id(&queue, current_index);
        self.status.queue = queue;
        self.status.current_index = current_index;
        self.status.current_position_ms = current_position_ms;
        self.status.current_song_id = next_song_id;
        self.status.playing_state = if self.status.queue.is_empty() {
            PlayingState::Idle
        } else {
            PlayingState::Stopped
        };
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.active_profile_id = Some(profile_id);

        log::info!(
            "controller.restore_state: restored queue_len={} current_index={:?} position_ms={}",
            self.status.queue.len(),
            self.status.current_index,
            self.status.current_position_ms,
        );

        self.report_status();
        self.state()
    }

    pub fn capabilities(&self) -> PlaybackCapabilities {
        self.playback_capabilities.clone()
    }

    pub fn state(&mut self) -> PlaybackStatus {
        self.update_gapless_status();
        self.status.clone()
    }

    pub fn synced_state(&mut self) -> Result<PlaybackStatus, String> {
        self.synced_state_with_context(None)
    }

    pub fn synced_state_with_context(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<PlaybackStatus, String> {
        self.process_native_events_inner(context, false)?;
        self.sync_current_position_from_backend()?;
        if let Err(error) = self.sync_gapless_for_current_track(context) {
            log::warn!("controller.synced_state_with_context: gapless refresh failed: {error}");
        }
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
        entries: Vec<SongResponse>,
        current_index: Option<u32>,
    ) -> Result<PlaybackStatus, String> {
        validate_queue_index(&entries, current_index)?;
        self.backend.stop()?;
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.clear_native_events();

        let next_song_id = current_song_id(&entries, current_index);
        self.status.queue = entries;
        self.status.current_index = current_index;
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
        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn set_position(&mut self, position_ms: u32) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        ) {
            self.backend.stop()?;
            self.clear_gapless_preparation();
            self.clear_native_events();
            self.clear_server_reporting();
            self.status.playing_state = PlayingState::Stopped;
        }

        self.status.current_song_id = current_song_id(&self.status.queue, Some(index));
        self.status.current_position_ms = position_ms;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;

        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn insert_after_current(
        &mut self,
        songs: Vec<SongResponse>,
    ) -> Result<PlaybackStatus, String> {
        if songs.is_empty() {
            return Ok(self.state());
        }

        self.clear_gapless_preparation();

        if self.status.queue.is_empty() {
            let next_song_id = current_song_id(&songs, Some(0));
            self.status.queue = songs;
            self.status.current_index = Some(0);
            self.status.current_song_id = next_song_id;
            self.status.playing_state = PlayingState::Stopped;
            log::info!(
                "controller.insert_after_current: initialized empty queue with {} songs",
                self.status.queue.len()
            );
        } else {
            let insert_pos = match self.status.current_index {
                Some(idx) => (idx as usize)
                    .saturating_add(1)
                    .min(self.status.queue.len()),
                None => 0,
            };
            let count = songs.len();
            for (i, song) in songs.into_iter().enumerate() {
                self.status.queue.insert(insert_pos + i, song);
            }
            log::info!(
                "controller.insert_after_current: inserted {count} songs at position {insert_pos}, queue_len={}",
                self.status.queue.len()
            );
        }

        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn append_to_queue(&mut self, songs: Vec<SongResponse>) -> Result<PlaybackStatus, String> {
        if songs.is_empty() {
            return Ok(self.state());
        }

        self.clear_gapless_preparation();

        if self.status.queue.is_empty() {
            let next_song_id = current_song_id(&songs, Some(0));
            self.status.queue = songs;
            self.status.current_index = Some(0);
            self.status.current_song_id = next_song_id;
            self.status.playing_state = PlayingState::Stopped;
            log::info!(
                "controller.append_to_queue: initialized empty queue with {} songs",
                self.status.queue.len()
            );
        } else {
            let count = songs.len();
            self.status.queue.extend(songs);
            log::info!(
                "controller.append_to_queue: appended {count} songs, queue_len={}",
                self.status.queue.len()
            );
        }

        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn play(&mut self, context: &PlaybackRuntimeContext<'_>) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;

        // If the track is already loaded and paused, resume without reloading.
        if self.status.playing_state == PlayingState::Paused {
            self.backend.resume()?;
            self.status.playing_state = PlayingState::Playing;
            self.status.error = None;
            self.status.interrupt_reason = None;
            self.process_native_events_inner(Some(context), false)?;
            if let Some(entry) =
                current_queue_entry(&self.status.queue, self.status.current_index).cloned()
            {
                self.report_server_playback_state(
                    context,
                    &entry,
                    self.status.current_position_ms,
                    ServerPlaybackState::Playing,
                );
            }
            self.report_status();
            return Ok(self.state());
        }

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

        self.persist_state();
        self.play(context)
    }

    pub fn pause(&mut self) -> Result<PlaybackStatus, String> {
        self.pause_with_context(None)
    }

    pub fn pause_with_context(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<PlaybackStatus, String> {
        self.sync_current_position_from_backend()?;
        self.backend.pause()?;
        self.status.playing_state = PlayingState::Paused;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.process_native_events_inner(None, false)?;
        self.persist_state();
        if let (Some(context), Some(entry)) = (
            context,
            current_queue_entry(&self.status.queue, self.status.current_index).cloned(),
        ) {
            self.report_server_playback_state(
                context,
                &entry,
                self.status.current_position_ms,
                ServerPlaybackState::Paused,
            );
        }
        self.report_status();
        Ok(self.state())
    }

    pub fn stop(&mut self) -> Result<PlaybackStatus, String> {
        self.stop_with_context(None)
    }

    pub fn stop_with_context(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<PlaybackStatus, String> {
        self.sync_current_position_from_backend()?;
        let stop_position_ms = self.status.current_position_ms;
        let stop_entry =
            current_queue_entry(&self.status.queue, self.status.current_index).cloned();
        self.clear_gapless_preparation();
        self.backend.stop()?;
        self.clear_native_events();
        if let (Some(context), Some(entry)) = (context, stop_entry.as_ref()) {
            self.report_server_playback_state(
                context,
                entry,
                stop_position_ms,
                ServerPlaybackState::Stopped,
            );
        }
        self.clear_server_reporting();
        self.status.playing_state = PlayingState::Stopped;
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.persist_state();
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
        if let Some(entry) =
            current_queue_entry(&self.status.queue, self.status.current_index).cloned()
        {
            let reported_position_ms = self
                .status
                .pending_seek_position_ms
                .unwrap_or(self.status.current_position_ms);
            match self.status.playing_state {
                PlayingState::Playing => self.report_server_playback_state(
                    context,
                    &entry,
                    reported_position_ms,
                    ServerPlaybackState::Playing,
                ),
                PlayingState::Paused => self.report_server_playback_state(
                    context,
                    &entry,
                    reported_position_ms,
                    ServerPlaybackState::Paused,
                ),
                _ => {}
            }
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

        self.persist_state();
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

        self.persist_state();
        self.process_native_events_inner(Some(context), false)?;
        self.report_status();
        Ok(self.state())
    }

    #[allow(dead_code)]
    pub fn on_track_finished(&mut self) -> PlaybackStatus {
        self.clear_gapless_preparation();
        self.clear_server_reporting();
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
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        if let Err(error) = self.backend.stop() {
            log::warn!("controller.reset: failed to stop backend: {error}");
        }

        self.clear_native_events();
        self.status = PlaybackStatus::empty();
        self.interrupted_resume_state = None;
        self.active_profile_id = None;
        self.sync_queue_state();
        self.persist_state();
        self.report_status();
        self.state()
    }

    /// Save the current playback state to persistent storage for later restoration.
    /// Only saves when an active profile_id is set. Failures are logged but not propagated.
    pub fn persist_state(&self) {
        let Some(profile_id) = &self.active_profile_id else {
            return;
        };

        let state_file = PlaybackStateFile {
            version: 1,
            profile_id: profile_id.clone(),
            queue: self.status.queue.clone(),
            current_index: self.status.current_index,
            current_position_ms: self.status.current_position_ms,
        };

        if let Err(error) = self.persister.save(&state_file) {
            log::warn!("controller.persist_state: failed to save: {error}");
        }
    }

    /// Sync position from backend and persist. Used for app exit handler.
    pub fn sync_and_persist(&mut self) {
        if self.active_profile_id.is_none() {
            return;
        }
        if matches!(self.status.playing_state, PlayingState::Playing) {
            let _ = self.sync_current_position_from_backend();
        }
        self.persist_state();
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
        self.report_server_playback_state(
            context,
            &entry,
            requested_position_ms,
            ServerPlaybackState::Starting,
        );

        let load_result = match self.try_activate_prepared_gapless_track(
            index,
            &entry,
            requested_position_ms,
            autoplay,
        ) {
            Ok(true) => Ok(()),
            Ok(false) => {
                self.load_stream_for_song(context, &entry, requested_position_ms, autoplay)
            }
            Err(error) => {
                log::warn!(
                    "controller.load_track_at_index: failed to activate prepared track for song_id={song_id}: {error}"
                );
                self.clear_gapless_preparation();
                self.load_stream_for_song(context, &entry, requested_position_ms, autoplay)
            }
        };

        if let Err(error) = load_result {
            self.status.playing_state = PlayingState::Error;
            self.status.error = Some(error.clone());
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.clear_server_reporting();
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
        if let Err(error) = self.sync_gapless_for_current_track(Some(context)) {
            log::warn!("controller.load_track_at_index: gapless refresh failed: {error}");
        }
        self.report_server_playback_state(
            context,
            &entry,
            requested_position_ms,
            if autoplay {
                ServerPlaybackState::Playing
            } else {
                ServerPlaybackState::Paused
            },
        );

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
        self.report_server_playback_state(
            context,
            &entry,
            requested_position_ms,
            ServerPlaybackState::Starting,
        );

        if let Err(error) =
            self.load_stream_for_song(context, &entry, requested_position_ms, autoplay)
        {
            self.status = previous_status;
            self.interrupted_resume_state = previous_resume_state;
            self.clear_server_reporting();
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
        if let Err(error) = self.sync_gapless_for_current_track(Some(context)) {
            log::warn!("controller.reload_track_at_index_exact: gapless refresh failed: {error}");
        }
        self.report_server_playback_state(
            context,
            &entry,
            requested_position_ms,
            if autoplay {
                ServerPlaybackState::Playing
            } else {
                ServerPlaybackState::Paused
            },
        );

        Ok(())
    }

    fn load_stream_for_song(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        entry: &SongResponse,
        requested_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String> {
        self.load_or_prepare_stream_for_song(context, entry, requested_position_ms, autoplay, false)
            .map(|_| ())
    }

    fn prepare_stream_for_song(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        entry: &SongResponse,
    ) -> Result<u64, String> {
        self.load_or_prepare_stream_for_song(context, entry, 0, false, true)
            .and_then(|generation| {
                generation.ok_or_else(|| "Missing gapless generation.".to_string())
            })
    }

    fn load_or_prepare_stream_for_song(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        entry: &SongResponse,
        requested_position_ms: u32,
        autoplay: bool,
        prepare_only: bool,
    ) -> Result<Option<u64>, String> {
        let song_id = entry.id.as_str();
        let load_strategy = self.backend.plan_load(
            requested_position_ms,
            context.capability_matrix.transcode_offset,
        );
        let stream_offset_seconds = load_strategy.stream_offset_seconds;
        let local_start_position_ms = load_strategy.local_start_position_ms;

        log::info!(
            "controller.{}: song_id={} path={:?} requested_position_ms={} server_offset_capability={} stream_offset_seconds={:?} local_start_position_ms={} autoplay={}",
            if prepare_only {
                "prepare_stream_for_song"
            } else {
                "load_stream_for_song"
            },
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
            let raw_request = self.build_backend_load_request(
                entry,
                raw_stream,
                artwork_path.clone(),
                requested_position_ms,
                local_start_position_ms,
                autoplay,
            );
            return match if prepare_only {
                self.backend.prepare(raw_request).map(Some)
            } else {
                self.backend.load(raw_request).map(|_| None)
            } {
                Ok(result) => Ok(result),
                Err(raw_error) => {
                    log::warn!(
                        "controller.{}: raw stream request failed for song_id={song_id}: {raw_error}",
                        if prepare_only {
                            "prepare_stream_for_song"
                        } else {
                            "load_track_at_index"
                        }
                    );
                    let fallback_stream = build_stream_request(
                        context.client,
                        song_id,
                        stream_offset_seconds,
                        false,
                    )?;
                    let fallback_request = self.build_backend_load_request(
                        entry,
                        fallback_stream,
                        artwork_path,
                        requested_position_ms,
                        local_start_position_ms,
                        autoplay,
                    );
                    match if prepare_only {
                        self.backend.prepare(fallback_request).map(Some)
                    } else {
                        self.backend.load(fallback_request).map(|_| None)
                    } {
                        Ok(result) => Ok(result),
                        Err(fallback_error) => {
                            let message = format!(
                                "Failed to {} playback stream. raw stream failed: {raw_error}; fallback stream failed: {fallback_error}",
                                if prepare_only { "prepare" } else { "load" }
                            );
                            log::error!(
                                "controller.{}: fallback stream request failed for song_id={song_id}: {fallback_error}",
                                if prepare_only {
                                    "prepare_stream_for_song"
                                } else {
                                    "load_track_at_index"
                                }
                            );
                            Err(message)
                        }
                    }
                }
            };
        }

        let standard_stream =
            build_stream_request(context.client, song_id, stream_offset_seconds, false)?;
        let standard_request = self.build_backend_load_request(
            entry,
            standard_stream,
            artwork_path,
            requested_position_ms,
            local_start_position_ms,
            autoplay,
        );
        if prepare_only {
            self.backend.prepare(standard_request).map(Some)
        } else {
            self.backend.load(standard_request).map(|_| None)
        }
        .map_err(|error| {
            format!(
                "Failed to {} playback stream: {error}",
                if prepare_only { "prepare" } else { "load" }
            )
        })
    }

    fn build_backend_load_request(
        &self,
        entry: &SongResponse,
        request: PreparedBinaryRequest,
        artwork_path: Option<String>,
        absolute_start_position_ms: u32,
        local_start_position_ms: u32,
        autoplay: bool,
    ) -> PlaybackBackendLoadRequest {
        PlaybackBackendLoadRequest {
            request,
            media_id: entry.id.clone(),
            title: entry.title.clone(),
            artist: entry.artist.clone(),
            album: entry.album.clone(),
            artwork_path,
            absolute_start_position_ms,
            local_start_position_ms,
            autoplay,
        }
    }

    fn try_activate_prepared_gapless_track(
        &mut self,
        index: u32,
        entry: &SongResponse,
        requested_position_ms: u32,
        autoplay: bool,
    ) -> Result<bool, String> {
        if !self.supports_gapless_playback() {
            return Ok(false);
        }

        let Some(prepared_state) = self.prepared_next.clone() else {
            return Ok(false);
        };

        let target = prepared_state.target();
        if requested_position_ms != 0 || target.queue_index != index || target.song_id != entry.id {
            self.clear_gapless_preparation();
            return Ok(false);
        }

        match prepared_state {
            PreparedGaplessTrackState::Ready { .. }
            | PreparedGaplessTrackState::Preparing { .. } => {
                self.try_activate_backend_prepared_track(autoplay)
            }
            PreparedGaplessTrackState::Failed { .. } => {
                self.clear_gapless_preparation();
                Ok(false)
            }
        }
    }

    fn try_activate_backend_prepared_track(&mut self, autoplay: bool) -> Result<bool, String> {
        match self.backend.activate_prepared(autoplay) {
            Ok(()) => {
                self.gapless_failure = None;
                self.prepared_next = None;
                self.update_gapless_status();
                Ok(true)
            }
            Err(error) if is_missing_prepared_stream_error(&error) => {
                self.clear_gapless_preparation();
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    fn sync_gapless_for_current_track(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<(), String> {
        if !self.supports_gapless_playback() {
            self.clear_gapless_preparation();
            return Ok(());
        }

        let Some(context) = context else {
            self.clear_gapless_preparation();
            return Ok(());
        };

        if !self.gapless_playback_enabled || !self.can_prepare_gapless_from_current_state() {
            self.clear_gapless_preparation();
            return Ok(());
        }

        let Some(target) = self.current_gapless_target() else {
            self.clear_gapless_preparation();
            return Ok(());
        };

        if self
            .prepared_next
            .as_ref()
            .is_some_and(|prepared| prepared.target() == &target)
        {
            return Ok(());
        }

        self.clear_gapless_preparation();
        let entry = current_queue_entry(&self.status.queue, Some(target.queue_index))
            .cloned()
            .ok_or_else(|| "Current playback entry does not exist.".to_string())?;

        match self.prepare_stream_for_song(context, &entry) {
            Ok(generation) => {
                self.gapless_failure = None;
                self.prepared_next =
                    Some(PreparedGaplessTrackState::Preparing { target, generation });
                self.update_gapless_status();
                Ok(())
            }
            Err(error) => {
                self.gapless_failure = Some(error.clone());
                self.prepared_next = Some(PreparedGaplessTrackState::Failed {
                    target,
                    generation: None,
                });
                self.update_gapless_status();
                Err(error)
            }
        }
    }

    fn can_prepare_gapless_from_current_state(&self) -> bool {
        matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        )
    }

    fn current_gapless_target(&self) -> Option<GaplessTrackTarget> {
        let current_index = self.status.current_index?;
        let next_index = current_index.checked_add(1)?;
        let next_entry = current_queue_entry(&self.status.queue, Some(next_index))?;
        Some(GaplessTrackTarget {
            queue_index: next_index,
            song_id: next_entry.id.clone(),
        })
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
            PlaybackNativeEvent::GaplessPrepared { generation } => {
                Ok(self.handle_gapless_prepared(generation))
            }
            PlaybackNativeEvent::GaplessFailed {
                generation,
                message,
            } => Ok(self.handle_gapless_failed(generation, message)),
            PlaybackNativeEvent::GaplessTransition => self.handle_gapless_transition(context),
            PlaybackNativeEvent::Ended => self.handle_track_ended(context),
        }
    }

    fn handle_gapless_prepared(&mut self, generation: u64) -> bool {
        let Some(PreparedGaplessTrackState::Preparing {
            target,
            generation: current_generation,
        }) = self.prepared_next.clone()
        else {
            return false;
        };
        if current_generation != generation {
            return false;
        }

        self.gapless_failure = None;
        self.prepared_next = Some(PreparedGaplessTrackState::Ready { target, generation });
        true
    }

    fn handle_gapless_failed(&mut self, generation: u64, message: String) -> bool {
        let Some(prepared_state) = self.prepared_next.clone() else {
            return false;
        };
        if prepared_state.generation() != Some(generation) {
            return false;
        }

        self.gapless_failure = Some(message);
        self.prepared_next = Some(PreparedGaplessTrackState::Failed {
            target: prepared_state.target().clone(),
            generation: Some(generation),
        });
        true
    }

    fn handle_gapless_transition(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<bool, String> {
        if !self.supports_gapless_playback() {
            self.clear_gapless_preparation();
            return Ok(false);
        }

        let Some(target) =
            self.prepared_next
                .clone()
                .and_then(|prepared_state| match prepared_state {
                    PreparedGaplessTrackState::Preparing { target, .. }
                    | PreparedGaplessTrackState::Ready { target, .. } => Some(target),
                    PreparedGaplessTrackState::Failed { .. } => None,
                })
        else {
            return Ok(false);
        };
        let Some(entry) = current_queue_entry(&self.status.queue, Some(target.queue_index)) else {
            self.clear_gapless_preparation();
            return Ok(false);
        };

        self.status.playing_state = PlayingState::Playing;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.current_index = Some(target.queue_index);
        self.status.current_song_id = Some(entry.id.clone());
        self.status.current_position_ms = 0;
        self.status.error = None;
        self.interrupted_resume_state = None;
        self.gapless_failure = None;
        self.prepared_next = None;

        if let Err(error) = self.sync_gapless_for_current_track(context) {
            log::warn!("controller.handle_gapless_transition: gapless refresh failed: {error}");
        }
        if let (Some(context), Some(entry)) = (
            context,
            current_queue_entry(&self.status.queue, self.status.current_index).cloned(),
        ) {
            self.report_server_playback_state(
                context,
                &entry,
                self.status.current_position_ms,
                ServerPlaybackState::Playing,
            );
        }

        Ok(true)
    }

    fn handle_track_ended(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<bool, String> {
        let ended_entry =
            current_queue_entry(&self.status.queue, self.status.current_index).cloned();
        let ended_position_ms = self.status.current_position_ms;
        let Some(current_index) = self.status.current_index else {
            return Ok(false);
        };
        let Some(next_index) = current_index.checked_add(1) else {
            self.transition_to_stopped_end_of_queue();
            if let (Some(context), Some(entry)) = (context, ended_entry.as_ref()) {
                self.report_server_playback_state(
                    context,
                    entry,
                    ended_position_ms,
                    ServerPlaybackState::Stopped,
                );
            }
            return Ok(true);
        };
        let has_next = usize::try_from(next_index)
            .ok()
            .is_some_and(|index| index < self.status.queue.len());
        if !has_next {
            self.transition_to_stopped_end_of_queue();
            if let (Some(context), Some(entry)) = (context, ended_entry.as_ref()) {
                self.report_server_playback_state(
                    context,
                    entry,
                    ended_position_ms,
                    ServerPlaybackState::Stopped,
                );
            }
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
        self.clear_gapless_preparation();
        self.clear_server_reporting();
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

    fn supports_gapless_playback(&self) -> bool {
        self.playback_capabilities.gapless_playback
    }

    fn update_gapless_status(&mut self) {
        self.status.gapless_status = if !self.supports_gapless_playback() {
            GaplessStatus {
                state: GaplessState::Unavailable,
                message: "gapless: unavailable".to_string(),
            }
        } else if !self.gapless_playback_enabled {
            GaplessStatus {
                state: GaplessState::Off,
                message: "gapless: off".to_string(),
            }
        } else if let Some(prepared) = &self.prepared_next {
            match prepared {
                PreparedGaplessTrackState::Preparing { target, .. } => GaplessStatus {
                    state: GaplessState::Preparing,
                    message: format!("gapless: preparing {}", self.gapless_target_label(target)),
                },
                PreparedGaplessTrackState::Ready { target, .. } => GaplessStatus {
                    state: GaplessState::Ready,
                    message: format!("gapless: ready {}", self.gapless_target_label(target)),
                },
                PreparedGaplessTrackState::Failed { target, .. } => {
                    let reason = self.gapless_failure.as_deref().unwrap_or("unknown error");
                    GaplessStatus {
                        state: GaplessState::Failed,
                        message: format!(
                            "gapless: failed {} ({reason})",
                            self.gapless_target_label(target)
                        ),
                    }
                }
            }
        } else if !self.can_prepare_gapless_from_current_state() {
            GaplessStatus {
                state: GaplessState::Idle,
                message: "gapless: idle (waiting for playback)".to_string(),
            }
        } else if let Some(target) = self.current_gapless_target() {
            GaplessStatus {
                state: GaplessState::Idle,
                message: format!("gapless: idle {}", self.gapless_target_label(&target)),
            }
        } else {
            GaplessStatus {
                state: GaplessState::Idle,
                message: "gapless: idle (no next track)".to_string(),
            }
        };
    }

    fn gapless_target_label(&self, target: &GaplessTrackTarget) -> String {
        current_queue_entry(&self.status.queue, Some(target.queue_index))
            .map(|entry| {
                format!(
                    "(#{} {})",
                    target.queue_index.saturating_add(1),
                    entry.title.as_str()
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "(#{} {})",
                    target.queue_index.saturating_add(1),
                    target.song_id
                )
            })
    }

    fn sync_queue_state(&mut self) {
        let _ = self.queue_sync.sync_queue(&self.status);
    }

    fn clear_server_reporting(&mut self) {
        self.server_reporter.clear();
    }

    fn report_server_playback_state(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        entry: &SongResponse,
        position_ms: u32,
        state: ServerPlaybackState,
    ) {
        let Some(profile_id) = context.profile_id else {
            return;
        };

        self.server_reporter.report_state(
            ServerPlaybackReportingContext {
                client: context.client.clone(),
                capability_matrix: context.capability_matrix.clone(),
                profile_id: profile_id.to_string(),
            },
            ServerPlaybackTrack {
                song_id: entry.id.clone(),
                media_type: server_playback_media_type(entry),
            },
            position_ms,
            state,
        );
    }

    fn report_status(&mut self) {
        self.update_gapless_status();
        let _ = self.reporter.report_state(&self.status);
    }
}

fn server_playback_media_type(entry: &SongResponse) -> MediaType {
    match entry.media_type.as_deref() {
        Some("podcast") => MediaType::Podcast,
        _ => MediaType::Song,
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
            estimate_content_length: Some(true),
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

fn is_missing_prepared_stream_error(error: &str) -> bool {
    error.to_ascii_lowercase().contains("no prepared stream")
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
            CapabilityMatrix, GaplessState, InterruptReason, PlaybackCapabilities, PlayingState,
            SongResponse,
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
            server_reporting::{
                NoopPlaybackServerReporter, PlaybackServerReporter, ServerPlaybackReportingContext,
                ServerPlaybackState, ServerPlaybackTrack,
            },
        },
        playback_state::NoopPlaybackStatePersister,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    enum MockSeekBehavior {
        #[default]
        Apply,
        ApplyAndEmitSeekProcessed,
        ReloadRequired,
        Fail,
    }

    #[derive(Debug, Default)]
    struct MockBackendState {
        load_calls: Vec<String>,
        load_absolute_start_positions_ms: Vec<u32>,
        load_local_start_positions_ms: Vec<u32>,
        prepare_calls: Vec<String>,
        next_prepare_generation: u64,
        seek_calls_ms: Vec<u32>,
        current_position_ms: u32,
        current_position_calls: usize,
        fail_first_load: bool,
        always_fail_load: bool,
        fail_standard_load: bool,
        seek_behavior: MockSeekBehavior,
        has_failed_first_load: bool,
        pause_calls: usize,
        resume_calls: usize,
        stop_calls: usize,
        activate_prepared_calls: usize,
        clear_prepared_calls: usize,
        activated_prepared_urls: Vec<String>,
        prepared_request: Option<MockPreparedRequest>,
    }

    #[derive(Debug, Clone)]
    struct MockPreparedRequest {
        url: String,
        absolute_start_position_ms: u32,
    }

    #[derive(Debug, Default)]
    struct MockReporterState {
        reported_states: Vec<PlaybackStatus>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockServerReportEvent {
        profile_id: String,
        song_id: String,
        position_ms: u32,
        state: ServerPlaybackState,
    }

    #[derive(Debug, Default)]
    struct MockServerReporterState {
        reported_events: Vec<MockServerReportEvent>,
    }

    struct MockBackend {
        state: Arc<Mutex<MockBackendState>>,
        playback_capabilities: PlaybackCapabilities,
        native_events: Option<Arc<Mutex<Vec<PlaybackNativeEvent>>>>,
    }

    struct MockReporter {
        state: Arc<Mutex<MockReporterState>>,
    }

    struct MockServerReporter {
        state: Arc<Mutex<MockServerReporterState>>,
    }

    #[derive(Default)]
    struct MockNativeEventSource {
        events: Arc<Mutex<Vec<PlaybackNativeEvent>>>,
    }

    impl PlaybackBackend for MockBackend {
        fn capabilities(&self) -> PlaybackCapabilities {
            self.playback_capabilities.clone()
        }

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

        fn prepare(&mut self, request: PlaybackBackendLoadRequest) -> Result<u64, String> {
            let mut state = self.state.lock().unwrap();
            let url = request.request.url.to_string();
            state.prepare_calls.push(url.clone());
            if state.always_fail_load {
                return Err("stream rejected".to_string());
            }
            if state.fail_standard_load && !url.contains("format=raw") {
                return Err("standard stream rejected".to_string());
            }
            if state.fail_first_load && !state.has_failed_first_load {
                state.has_failed_first_load = true;
                return Err("raw stream rejected".to_string());
            }

            state.next_prepare_generation = state.next_prepare_generation.wrapping_add(1);
            let generation = state.next_prepare_generation;
            state.prepared_request = Some(MockPreparedRequest {
                url,
                absolute_start_position_ms: request.absolute_start_position_ms,
            });
            Ok(generation)
        }

        fn activate_prepared(&mut self, _autoplay: bool) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            let Some(request) = state.prepared_request.take() else {
                return Err("no prepared stream".to_string());
            };
            state.activate_prepared_calls += 1;
            state.activated_prepared_urls.push(request.url);
            state.current_position_ms = request.absolute_start_position_ms;
            Ok(())
        }

        fn clear_prepared(&mut self) {
            let mut state = self.state.lock().unwrap();
            state.clear_prepared_calls += 1;
            state.prepared_request = None;
        }

        fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String> {
            let mut state = self.state.lock().unwrap();
            state.seek_calls_ms.push(position_ms);
            match state.seek_behavior {
                MockSeekBehavior::Apply => {
                    state.current_position_ms = position_ms;
                    Ok(PlaybackSeekAction::Applied)
                }
                MockSeekBehavior::ApplyAndEmitSeekProcessed => {
                    state.current_position_ms = position_ms;
                    if let Some(native_events) = &self.native_events {
                        native_events
                            .lock()
                            .unwrap()
                            .push(PlaybackNativeEvent::SeekProcessed { position_ms });
                    }
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

        fn resume(&mut self) -> Result<(), String> {
            self.state.lock().unwrap().resume_calls += 1;
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

    impl PlaybackServerReporter for MockServerReporter {
        fn report_state(
            &mut self,
            context: ServerPlaybackReportingContext,
            track: ServerPlaybackTrack,
            position_ms: u32,
            state: ServerPlaybackState,
        ) {
            self.state
                .lock()
                .unwrap()
                .reported_events
                .push(MockServerReportEvent {
                    profile_id: context.profile_id,
                    song_id: track.song_id,
                    position_ms,
                    state,
                });
        }

        fn clear(&mut self) {}
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
        runtime_context_with_reporting(transcode_offset, false)
    }

    fn runtime_context_with_reporting(
        transcode_offset: bool,
        playback_report: bool,
    ) -> (opensubsonic_client::OpenSubsonicClient, CapabilityMatrix) {
        let mut capability_matrix = CapabilityMatrix::empty();
        capability_matrix.transcode_offset = transcode_offset;
        capability_matrix.playback_report = playback_report;
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
            playback_capabilities: PlaybackCapabilities {
                gapless_playback: true,
            },
            native_events: None,
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state.clone(),
        });
        let server_reporter: Box<dyn PlaybackServerReporter> = Box::new(NoopPlaybackServerReporter);
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let controller = PlaybackController::new(
            backend,
            reporter,
            server_reporter,
            queue_sync,
            Box::new(NoopNativePlaybackEventSource),
            Box::new(NoopPlaybackStatePersister),
            false,
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
            playback_capabilities: PlaybackCapabilities {
                gapless_playback: true,
            },
            native_events: Some(native_events.clone()),
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state.clone(),
        });
        let server_reporter: Box<dyn PlaybackServerReporter> = Box::new(NoopPlaybackServerReporter);
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let controller = PlaybackController::new(
            backend,
            reporter,
            server_reporter,
            queue_sync,
            Box::new(MockNativeEventSource {
                events: native_events.clone(),
            }),
            Box::new(NoopPlaybackStatePersister),
            false,
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

    fn controller_with_mock_backend_capabilities(
        gapless_playback: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities: PlaybackCapabilities { gapless_playback },
            native_events: None,
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state.clone(),
        });
        let server_reporter: Box<dyn PlaybackServerReporter> = Box::new(NoopPlaybackServerReporter);
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let controller = PlaybackController::new(
            backend,
            reporter,
            server_reporter,
            queue_sync,
            Box::new(NoopNativePlaybackEventSource),
            Box::new(NoopPlaybackStatePersister),
            false,
        );
        (controller, state, reporter_state)
    }

    fn controller_with_mock_backend_and_server_reporter(
        playback_report: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockServerReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let server_reporter_state = Arc::new(Mutex::new(MockServerReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities: PlaybackCapabilities {
                gapless_playback: true,
            },
            native_events: None,
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state,
        });
        let server_reporter: Box<dyn PlaybackServerReporter> = Box::new(MockServerReporter {
            state: server_reporter_state.clone(),
        });
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(NoopQueueSyncGateway);
        let mut controller = PlaybackController::new(
            backend,
            reporter,
            server_reporter,
            queue_sync,
            Box::new(NoopNativePlaybackEventSource),
            Box::new(NoopPlaybackStatePersister),
            false,
        );
        if playback_report {
            controller.set_active_profile_id(Some("profile-1".to_string()));
        }
        (controller, state, server_reporter_state)
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

        let result = controller.set_queue(queue_entries(&["song-a"]), Some(1));

        assert!(result.is_err());
    }

    #[test]
    fn duplicate_song_ids_are_distinguished_by_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let status = controller
            .set_queue(queue_entries(&["song-a", "song-a", "song-b"]), Some(1))
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
    fn server_reporting_uses_explicit_playback_transition_points() {
        let (mut controller, backend_state, server_reporter_state) =
            controller_with_mock_backend_and_server_reporter(true);
        let (client, capability_matrix) = runtime_context_with_reporting(true, true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: Some("profile-1"),
        };
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();
        backend_state.lock().unwrap().current_position_ms = 2_500;
        controller
            .pause_with_context(Some(&runtime_context))
            .unwrap();
        controller.seek(&runtime_context, 8_000).unwrap();
        controller
            .stop_with_context(Some(&runtime_context))
            .unwrap();

        let events = server_reporter_state
            .lock()
            .unwrap()
            .reported_events
            .clone();
        assert_eq!(
            events,
            vec![
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-a".to_string(),
                    position_ms: 0,
                    state: ServerPlaybackState::Starting,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-a".to_string(),
                    position_ms: 0,
                    state: ServerPlaybackState::Playing,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-a".to_string(),
                    position_ms: 2_500,
                    state: ServerPlaybackState::Paused,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-a".to_string(),
                    position_ms: 8_000,
                    state: ServerPlaybackState::Paused,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-a".to_string(),
                    position_ms: 8_000,
                    state: ServerPlaybackState::Stopped,
                },
            ]
        );
    }

    #[test]
    fn play_from_paused_resumes_without_reloading() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(false);
        let context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["a", "b"]), Some(0))
            .unwrap();
        controller.play(&context).unwrap();
        controller.pause().unwrap();

        // Resume from paused — should NOT trigger a new load.
        let load_count_before = backend_state.lock().unwrap().load_calls.len();
        controller.play(&context).unwrap();
        let state = backend_state.lock().unwrap();
        assert_eq!(state.load_calls.len(), load_count_before);
        assert_eq!(state.resume_calls, 1);
        drop(state);

        assert_eq!(controller.state().playing_state, PlayingState::Playing);
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
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
    fn enabling_gapless_prepares_next_track() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();

        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.prepare_calls.len(), 1);
        assert!(backend_state.prepare_calls[0].contains("song-b"));
        assert!(matches!(
            status.gapless_status.state,
            GaplessState::Preparing
        ));
    }

    #[test]
    fn manual_next_uses_prepared_gapless_track_when_ready() {
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::GaplessPrepared { generation: 1 });
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        controller.next(&runtime_context).unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.activate_prepared_calls, 1);
        assert_eq!(controller.state().current_index, Some(1));
        assert_eq!(
            controller.state().current_song_id.as_deref(),
            Some("song-b")
        );
    }

    #[test]
    fn manual_next_uses_prepared_gapless_track_before_ready_event_arrives() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();

        controller.next(&runtime_context).unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.activate_prepared_calls, 1);
        assert_eq!(controller.state().current_index, Some(1));
        assert_eq!(
            controller.state().current_song_id.as_deref(),
            Some("song-b")
        );
    }

    #[test]
    fn ended_native_event_uses_prepared_gapless_track_when_ready() {
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::GaplessPrepared { generation: 1 });
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Ended);

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.activate_prepared_calls, 1);
        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-b"));
    }

    #[test]
    fn gapless_transition_native_event_advances_without_backend_activation() {
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
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::GaplessPrepared { generation: 1 });
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::GaplessTransition);

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.activate_prepared_calls, 0);
        assert_eq!(backend_state.prepare_calls.len(), 2);

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-b"));
        assert!(matches!(
            status.gapless_status.state,
            GaplessState::Preparing
        ));
    }

    #[test]
    fn gapless_transition_native_event_advances_without_ready_signal() {
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
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::GaplessTransition);

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.load_calls.len(), 1);
        assert_eq!(backend_state.activate_prepared_calls, 0);
        assert_eq!(backend_state.prepare_calls.len(), 2);

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-b"));
        assert!(matches!(
            status.gapless_status.state,
            GaplessState::Preparing
        ));
    }

    #[test]
    fn unsupported_backend_reports_gapless_unavailable_and_skips_prepare() {
        let (mut controller, backend_state, _) = controller_with_mock_backend_capabilities(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller
            .set_gapless_playback_enabled(true, Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.prepare_calls.len(), 0);
        assert_eq!(status.gapless_status.state, GaplessState::Unavailable);
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
    fn native_seek_processed_event_queued_during_seek_clears_pending_before_seek_returns() {
        let (mut controller, backend_state, _, _) =
            controller_with_mock_backend_and_native_events(false);
        backend_state.lock().unwrap().seek_behavior = MockSeekBehavior::ApplyAndEmitSeekProcessed;
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();

        let status = controller.seek(&runtime_context, 5_500).unwrap();

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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller.seek(&runtime_context, 5_500).unwrap();

        let status = controller
            .set_queue(queue_entries(&["song-b"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
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
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();

        let status = controller.reset();
        assert_eq!(status.playing_state, PlayingState::Idle);
        assert!(status.queue.is_empty());
        assert_eq!(status.gapless_status.state, GaplessState::Off);

        let reported_states = reporter_state.lock().unwrap().reported_states.clone();
        assert_eq!(
            reported_states
                .last()
                .map(|state| state.playing_state.clone()),
            Some(PlayingState::Idle)
        );
        assert_eq!(
            reported_states
                .last()
                .map(|state| state.gapless_status.state.clone()),
            Some(GaplessState::Off)
        );
    }

    #[test]
    fn insert_after_current_into_empty_queue_initializes_queue() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let songs = queue_entries(&["song-x", "song-y"]);
        let status = controller.insert_after_current(songs).unwrap();
        assert_eq!(status.queue.len(), 2);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_song_id, Some("song-x".to_string()));
        assert_eq!(status.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn insert_after_current_inserts_after_current_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(1))
            .unwrap();

        let status = controller
            .insert_after_current(queue_entries(&["song-x", "song-y"]))
            .unwrap();
        assert_eq!(status.queue.len(), 5);
        assert_eq!(status.queue[0].id, "song-a");
        assert_eq!(status.queue[1].id, "song-b"); // current
        assert_eq!(status.queue[2].id, "song-x"); // inserted
        assert_eq!(status.queue[3].id, "song-y"); // inserted
        assert_eq!(status.queue[4].id, "song-c");
        assert_eq!(status.current_index, Some(1)); // unchanged
    }

    #[test]
    fn insert_after_current_with_empty_songs_is_noop() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        let status = controller.insert_after_current(vec![]).unwrap();
        assert_eq!(status.queue.len(), 1);
    }

    #[test]
    fn append_to_queue_into_empty_queue_initializes_queue() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let songs = queue_entries(&["song-x", "song-y"]);
        let status = controller.append_to_queue(songs).unwrap();
        assert_eq!(status.queue.len(), 2);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_song_id, Some("song-x".to_string()));
        assert_eq!(status.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn append_to_queue_appends_to_end() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();

        let status = controller
            .append_to_queue(queue_entries(&["song-x", "song-y"]))
            .unwrap();
        assert_eq!(status.queue.len(), 4);
        assert_eq!(status.queue[2].id, "song-x");
        assert_eq!(status.queue[3].id, "song-y");
        assert_eq!(status.current_index, Some(0)); // unchanged
    }

    #[test]
    fn append_to_queue_with_empty_songs_is_noop() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        let status = controller.append_to_queue(vec![]).unwrap();
        assert_eq!(status.queue.len(), 1);
    }

    #[test]
    fn set_position_updates_stopped_queue_position() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        let status = controller.set_position(12_345).unwrap();

        assert_eq!(status.playing_state, PlayingState::Stopped);
        assert_eq!(status.current_position_ms, 12_345);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_song_id, Some("song-a".to_string()));
    }

    #[test]
    fn set_position_stops_loaded_paused_track() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        controller.pause().unwrap();
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        let status = controller.set_position(30_000).unwrap();

        assert_eq!(status.playing_state, PlayingState::Stopped);
        assert_eq!(status.current_position_ms, 30_000);
        assert_eq!(
            backend_state.lock().unwrap().stop_calls,
            stop_calls_before + 1
        );
    }

    #[test]
    fn restore_state_sets_queue_and_stopped_state() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let queue = queue_entries(&["song-a", "song-b", "song-c"]);
        let status = controller.restore_state("profile-1".to_string(), queue, Some(1), 5000);
        assert_eq!(status.queue.len(), 3);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-b".to_string()));
        assert_eq!(status.current_position_ms, 5000);
        assert_eq!(status.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn restore_state_with_empty_queue_sets_idle() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let status = controller.restore_state("profile-1".to_string(), vec![], None, 0);
        assert_eq!(status.playing_state, PlayingState::Idle);
        assert_eq!(status.queue.len(), 0);
    }

    #[test]
    fn restore_state_with_invalid_index_ignores_queue() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let queue = queue_entries(&["song-a"]);
        let status = controller.restore_state("profile-1".to_string(), queue, Some(5), 0);
        // Should start fresh - queue not restored
        assert_eq!(status.queue.len(), 0);
        assert_eq!(status.playing_state, PlayingState::Idle);
    }
}
