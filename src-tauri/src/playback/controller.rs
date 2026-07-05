use std::time::{Duration, Instant};

use opensubsonic_client::{
    api::retrieval::{RetrievalApi, StreamRequest},
    ApiError, MediaType, OpenSubsonicClient, PreparedBinaryRequest,
};

use crate::{
    cover_art_cache::CoverArtCache,
    models::{
        normalize_output_device_id, normalize_volume, CapabilityMatrix, ConnectPlaybackState,
        GaplessState, GaplessStatus, InterruptReason, PlaybackCapabilities, PlaybackError,
        PlaybackStatus, PlaybackStreamInfo, PlaybackStreamMode, PlaybackStreamRequestKind,
        PlaybackTranscodingCodec, PlayingState, SongResponse,
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
    #[allow(dead_code)]
    pub cover_art_cache: Option<&'a CoverArtCache>,
    pub profile_id: Option<&'a str>,
}

const HANDLED_PLAYBACK_ERROR_PREFIX: &str = "[transonic-handled-playback-error]";
const OUTPUT_DEVICE_ERROR_PREFIX: &str = "Audio output device error:";
const OUTPUT_STREAM_RECOVERY_MAX_ATTEMPTS: u8 = 3;
/// Standard "previous" UX threshold: if playback has progressed past this many
/// milliseconds into the current track, `prev` restarts the current track from the
/// beginning instead of moving to the previous track. Mirrors the de-facto industry
/// default (Media3's `maxSeekToPreviousPositionMs`, Spotify/Apple Music, etc.).
const PREV_RESTART_THRESHOLD_MS: u32 = 3000;

fn output_stream_recovery_delay(attempt: u8) -> Duration {
    #[cfg(test)]
    {
        let _ = attempt;
        Duration::ZERO
    }
    #[cfg(not(test))]
    {
        Duration::from_millis(match attempt {
            0 | 1 => 100,
            2 => 500,
            _ => 1_500,
        })
    }
}

fn playback_error_from_load_failure(error: &str) -> PlaybackError {
    PlaybackError {
        message: error.replace(HANDLED_PLAYBACK_ERROR_PREFIX, ""),
        handled: error.contains(HANDLED_PLAYBACK_ERROR_PREFIX),
    }
}

fn is_output_device_load_error(error: &str) -> bool {
    error.contains(OUTPUT_DEVICE_ERROR_PREFIX)
}

fn is_output_stream_runtime_error(error: &str) -> bool {
    is_output_device_load_error(error)
        && (error.contains("output stream lost") || error.contains("output stream failed"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
pub struct PlaybackArtworkUpdateTarget {
    pub queue_index: u32,
    pub song_id: String,
    pub cover_art_id: String,
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

#[derive(Debug, Clone)]
struct OutputStreamRecoveryPlan {
    message: String,
    index: u32,
    requested_position_ms: u32,
    autoplay: bool,
    attempt: u8,
    not_before: Instant,
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
    stream_mode: PlaybackStreamMode,
    transcoding_max_bit_rate: u32,
    transcoding_codec: PlaybackTranscodingCodec,
    output_device_id: Option<String>,
    output_stream_recovery_attempts: u8,
    pending_output_stream_recovery: Option<OutputStreamRecoveryPlan>,
    prepared_next: Option<PreparedGaplessTrackState>,
    gapless_failure: Option<String>,
    status: PlaybackStatus,
    interrupted_resume_state: Option<PlayingState>,
    active_profile_id: Option<String>,
    restore_state_deferred: bool,
}

impl PlaybackController {
    pub fn new(
        mut backend: Box<dyn PlaybackBackend>,
        reporter: Box<dyn PlaybackReporter>,
        server_reporter: Box<dyn PlaybackServerReporter>,
        queue_sync: Box<dyn QueueSyncGateway>,
        native_events: Box<dyn NativePlaybackEventSource>,
        persister: Box<dyn PlaybackStatePersister>,
        gapless_playback_enabled: bool,
        initial_volume: f32,
        initial_output_device_id: Option<String>,
    ) -> Self {
        let playback_capabilities = backend.capabilities();
        let volume = normalize_volume(initial_volume);
        if let Err(error) = backend.set_volume(volume) {
            log::warn!("controller.new: failed to apply initial volume: {error}");
        }
        let output_device_id = normalize_output_device_id(initial_output_device_id);
        if let Err(error) = backend.set_output_device(output_device_id.clone()) {
            log::warn!("controller.new: failed to apply initial output device: {error}");
        }
        Self {
            backend,
            reporter,
            server_reporter,
            queue_sync,
            native_events,
            persister,
            playback_capabilities,
            gapless_playback_enabled,
            stream_mode: PlaybackStreamMode::Raw,
            transcoding_max_bit_rate: 320,
            transcoding_codec: PlaybackTranscodingCodec::Mp3,
            output_device_id,
            output_stream_recovery_attempts: 0,
            pending_output_stream_recovery: None,
            prepared_next: None,
            gapless_failure: None,
            status: PlaybackStatus::empty(),
            interrupted_resume_state: None,
            active_profile_id: None,
            restore_state_deferred: false,
        }
    }

    pub fn set_active_profile_id(&mut self, profile_id: Option<String>) {
        if self.active_profile_id != profile_id {
            self.clear_gapless_preparation();
            self.clear_server_reporting();
        }
        self.active_profile_id = profile_id;
        self.restore_state_deferred = false;
    }

    pub fn defer_state_restore(&mut self, profile_id: String) {
        if self.active_profile_id.as_deref() != Some(profile_id.as_str()) {
            self.clear_gapless_preparation();
            self.clear_server_reporting();
        }
        self.active_profile_id = Some(profile_id);
        self.restore_state_deferred = true;
    }

    pub fn is_initialized_for_profile(&self, profile_id: &str) -> bool {
        self.active_profile_id.as_deref() == Some(profile_id) && !self.restore_state_deferred
    }

    pub fn can_preserve_active_connect_snapshot_for_profile(&self, profile_id: &str) -> bool {
        self.is_initialized_for_profile(profile_id) && !self.status.queue.is_empty()
    }

    pub fn clear_gapless_preparation(&mut self) {
        if self.prepared_next.is_some() || self.status.prepared_stream_info.is_some() {
            self.backend.clear_prepared();
        }
        self.prepared_next = None;
        self.status.prepared_stream_info = None;
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

    pub fn set_stream_settings(
        &mut self,
        mode: PlaybackStreamMode,
        transcoding_max_bit_rate: u32,
        transcoding_codec: PlaybackTranscodingCodec,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<(), String> {
        if self.stream_mode == mode
            && self.transcoding_max_bit_rate == transcoding_max_bit_rate
            && self.transcoding_codec == transcoding_codec
        {
            return Ok(());
        }
        self.stream_mode = mode;
        self.transcoding_max_bit_rate = transcoding_max_bit_rate;
        self.transcoding_codec = transcoding_codec;
        self.clear_gapless_preparation();
        let result = self.sync_gapless_for_current_track(context);
        self.report_status();
        result
    }

    pub fn set_output_device(
        &mut self,
        output_device_id: Option<String>,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<(), String> {
        let output_device_id = normalize_output_device_id(output_device_id);
        if self.output_device_id == output_device_id {
            return Ok(());
        }
        let previous_output_device_id = self.output_device_id.clone();

        let should_reload = matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        );
        let reload_context = if should_reload {
            Some(context.ok_or_else(|| {
                "No active session is available to reload playback after output device change."
                    .to_string()
            })?)
        } else {
            None
        };

        let reload_index = if should_reload {
            let index = self.ensure_current_index()?;
            self.sync_current_position_from_backend()?;
            Some(index)
        } else {
            None
        };
        let reload_position_ms = self.status.current_position_ms;
        let autoplay = matches!(self.status.playing_state, PlayingState::Playing);

        self.backend.set_output_device(output_device_id.clone())?;
        self.output_device_id = output_device_id;
        self.clear_gapless_preparation();

        if let Some(index) = reload_index {
            if let Err(error) = self.backend.stop() {
                let rollback_error = self
                    .backend
                    .set_output_device(previous_output_device_id.clone())
                    .err();
                self.output_device_id = previous_output_device_id;
                return Err(format_output_device_change_error(error, rollback_error));
            }
            self.clear_native_events();
            self.clear_server_reporting();
            self.status.actual_stream_info = None;
            let reload_result = self.reload_track_at_index_exact(
                reload_context.expect("reload context is present when reload_index is set"),
                index,
                reload_position_ms,
                autoplay,
            );
            if let Err(error) = reload_result {
                let rollback_error = self
                    .backend
                    .set_output_device(previous_output_device_id.clone())
                    .err();
                self.output_device_id = previous_output_device_id;
                let playback_error = playback_error_from_load_failure(&error);
                let returned_error = playback_error.message.clone();
                self.status.playing_state = PlayingState::Error;
                self.status.current_position_ms = reload_position_ms;
                self.status.playback_error = Some(playback_error);
                self.status.interrupt_reason = None;
                self.status.pending_seek_position_ms = None;
                self.status.actual_stream_info = None;
                self.interrupted_resume_state = None;
                self.clear_gapless_preparation();
                self.clear_server_reporting();
                self.report_status();
                return Err(format_output_device_change_error(
                    returned_error,
                    rollback_error,
                ));
            }
        }

        self.reset_output_stream_recovery_attempts();
        self.persist_state();
        self.report_status();
        Ok(())
    }

    pub fn restore_state(
        &mut self,
        profile_id: String,
        queue: Vec<SongResponse>,
        current_index: Option<u32>,
        play_next_queue_len: u32,
        current_position_ms: u32,
    ) -> PlaybackStatus {
        if let Err(error) = validate_queue_index(&queue, current_index) {
            log::warn!(
                "controller.restore_state: invalid persisted index, starting fresh: {error}"
            );
            self.active_profile_id = Some(profile_id);
            self.restore_state_deferred = false;
            return self.state();
        }

        let next_song_id = current_song_id(&queue, current_index);
        self.status.queue = queue;
        self.status.current_index = current_index;
        self.status.play_next_queue_len = play_next_queue_len;
        clamp_play_next_queue_len(&mut self.status);
        self.status.current_position_ms = current_position_ms;
        self.status.current_song_id = next_song_id;
        self.status.playing_state = if self.status.queue.is_empty() {
            PlayingState::Idle
        } else {
            PlayingState::Stopped
        };
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.active_profile_id = Some(profile_id);
        self.restore_state_deferred = false;

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

    pub fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        let volume = normalize_volume(volume);
        self.backend.set_volume(volume)?;
        Ok(())
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
        self.sync_actual_stream_info_from_backend();
        if let Err(error) = self.sync_gapless_for_current_track(context) {
            log::warn!("controller.synced_state_with_context: gapless refresh failed: {error}");
        }
        Ok(self.state())
    }

    #[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
    pub fn current_artwork_update_target(&self) -> Option<PlaybackArtworkUpdateTarget> {
        let queue_index = self.status.current_index?;
        let entry = current_queue_entry(&self.status.queue, Some(queue_index))?;
        let song_id = self.status.current_song_id.as_ref()?;
        if entry.id != *song_id {
            return None;
        }

        Some(PlaybackArtworkUpdateTarget {
            queue_index,
            song_id: song_id.clone(),
            cover_art_id: display_cover_art_id(entry)?.to_string(),
        })
    }

    #[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
    pub fn update_artwork_if_current(
        &mut self,
        target: &PlaybackArtworkUpdateTarget,
        artwork_path: String,
    ) -> Result<bool, String> {
        let Some(current_target) = self.current_artwork_update_target() else {
            return Ok(false);
        };
        if &current_target != target {
            return Ok(false);
        }

        self.backend
            .update_artwork(&target.song_id, Some(artwork_path))?;
        Ok(true)
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
        self.restore_state_deferred = false;
        self.backend.stop()?;
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.clear_native_events();

        let next_song_id = current_song_id(&entries, current_index);
        self.status.queue = entries;
        self.status.current_index = current_index;
        self.status.play_next_queue_len = 0;
        self.status.current_position_ms = 0;
        self.status.current_song_id = next_song_id;
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
        self.interrupted_resume_state = None;
        self.reset_output_stream_recovery_attempts();
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

    pub fn clear_queue(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<PlaybackStatus, String> {
        self.sync_current_position_from_backend()?;
        self.restore_state_deferred = false;
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

        self.status.queue.clear();
        self.status.current_index = None;
        self.status.play_next_queue_len = 0;
        self.status.current_position_ms = 0;
        self.status.current_song_id = None;
        self.status.playing_state = PlayingState::Idle;
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
        self.interrupted_resume_state = None;
        self.reset_output_stream_recovery_attempts();

        self.sync_queue_state();
        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn set_position(&mut self, position_ms: u32) -> Result<PlaybackStatus, String> {
        let index = self.ensure_current_index()?;
        self.restore_state_deferred = false;
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
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.sync_actual_stream_info_from_backend();
        self.interrupted_resume_state = None;
        self.reset_output_stream_recovery_attempts();

        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn apply_connect_shared_state(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
        state: ConnectPlaybackState,
        is_active_device: bool,
        allow_preserve_active_track: bool,
    ) -> Result<PlaybackStatus, String> {
        validate_queue_index(&state.queue, state.current_index)?;
        if let Some(profile_id) = context.and_then(|context| context.profile_id) {
            self.set_active_profile_id(Some(profile_id.to_string()));
        }

        let desired_playing_state = state.playing_state.clone();
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        ) {
            if let Err(error) = self.sync_current_position_from_backend() {
                log::warn!(
                    "controller.apply_connect_shared_state: failed to sync position before apply: {error}"
                );
            }
        }

        if allow_preserve_active_track
            && self.should_preserve_active_track_for_shared_state(&state, is_active_device)
        {
            self.clear_gapless_preparation();
            self.status.queue = state.queue;
            self.status.current_index = state.current_index;
            self.status.play_next_queue_len = state.play_next_queue_len;
            clamp_play_next_queue_len(&mut self.status);
            self.status.current_song_id =
                current_song_id(&self.status.queue, self.status.current_index);
            self.status.playing_state = desired_playing_state;
            self.status.playback_error = None;
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.interrupted_resume_state = None;
            if let Err(error) = self.sync_gapless_for_current_track(context) {
                log::warn!(
                    "controller.apply_connect_shared_state: gapless refresh failed: {error}"
                );
            }
            self.sync_queue_state();
            self.persist_state();
            self.report_status();
            return Ok(self.state());
        }

        self.backend.stop()?;
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.clear_native_events();

        self.status.queue = state.queue;
        self.status.current_index = state.current_index;
        self.status.play_next_queue_len = state.play_next_queue_len;
        clamp_play_next_queue_len(&mut self.status);
        self.status.current_position_ms = state.current_position_ms;
        self.status.current_song_id =
            current_song_id(&self.status.queue, self.status.current_index);
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.sync_actual_stream_info_from_backend();
        self.interrupted_resume_state = None;

        if self.status.queue.is_empty() {
            self.status.playing_state = PlayingState::Idle;
            self.status.current_index = None;
            self.status.current_song_id = None;
            self.status.current_position_ms = 0;
            self.sync_queue_state();
            self.persist_state();
            self.report_status();
            return Ok(self.state());
        }

        if !is_active_device {
            self.status.playing_state = desired_playing_state;
            self.sync_queue_state();
            self.persist_state();
            self.report_status();
            return Ok(self.state());
        }

        self.status.playing_state = PlayingState::Stopped;
        self.sync_queue_state();
        self.persist_state();

        match desired_playing_state {
            PlayingState::Playing => {
                let context =
                    context.ok_or_else(|| "No active session is available.".to_string())?;
                self.play(context)
            }
            PlayingState::Paused => {
                let context =
                    context.ok_or_else(|| "No active session is available.".to_string())?;
                self.play(context)?;
                self.pause_with_context(Some(context))
            }
            PlayingState::Stopped | PlayingState::Idle => {
                self.status.playing_state = desired_playing_state;
                self.report_status();
                Ok(self.state())
            }
            PlayingState::Interrupted | PlayingState::Error => {
                self.status.playing_state = PlayingState::Stopped;
                self.report_status();
                Ok(self.state())
            }
        }
    }

    pub fn insert_after_current(
        &mut self,
        songs: Vec<SongResponse>,
    ) -> Result<PlaybackStatus, String> {
        if songs.is_empty() {
            return Ok(self.state());
        }

        self.restore_state_deferred = false;
        self.clear_gapless_preparation();
        self.sync_current_position_for_queue_mutation("insert_after_current");

        if self.status.queue.is_empty() {
            let next_song_id = current_song_id(&songs, Some(0));
            self.status.queue = songs;
            self.status.current_index = Some(0);
            self.status.play_next_queue_len =
                u32::try_from(self.status.queue.len().saturating_sub(1)).unwrap_or(u32::MAX);
            clamp_play_next_queue_len(&mut self.status);
            self.status.current_song_id = next_song_id;
            self.status.playing_state = PlayingState::Stopped;
            log::info!(
                "controller.insert_after_current: initialized empty queue with {} songs",
                self.status.queue.len()
            );
        } else {
            clamp_play_next_queue_len(&mut self.status);
            let insert_pos = play_next_end_index(&self.status);
            let count = songs.len();
            for (i, song) in songs.into_iter().enumerate() {
                self.status.queue.insert(insert_pos + i, song);
            }
            self.status.play_next_queue_len = self
                .status
                .play_next_queue_len
                .saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
            clamp_play_next_queue_len(&mut self.status);
            log::info!(
                "controller.insert_after_current: inserted {count} songs at play-next position {insert_pos}, queue_len={}",
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

        self.restore_state_deferred = false;
        self.clear_gapless_preparation();
        self.sync_current_position_for_queue_mutation("append_to_queue");

        if self.status.queue.is_empty() {
            let next_song_id = current_song_id(&songs, Some(0));
            self.status.queue = songs;
            self.status.current_index = Some(0);
            self.status.play_next_queue_len = 0;
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

    pub fn move_queue_index(
        &mut self,
        from_index: u32,
        to_index: u32,
    ) -> Result<PlaybackStatus, String> {
        let from_index = usize::try_from(from_index)
            .map_err(|_| "Playback queue index is out of range.".to_string())?;
        let to_index = usize::try_from(to_index)
            .map_err(|_| "Playback queue index is out of range.".to_string())?;
        if from_index >= self.status.queue.len() || to_index >= self.status.queue.len() {
            return Err("Playback queue index is out of range.".to_string());
        }
        if from_index == to_index {
            return Ok(self.state());
        }

        self.restore_state_deferred = false;
        let current_index = self
            .status
            .current_index
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < self.status.queue.len());
        let previous_current_index = self.status.current_index;
        let previous_play_next_queue_len = self.status.play_next_queue_len;

        self.clear_gapless_preparation();
        self.sync_current_position_for_queue_mutation("move_queue_index");

        let entry = self.status.queue.remove(from_index);
        self.status.queue.insert(to_index, entry);

        let next_current_index = match current_index {
            None => None,
            Some(current_index) if current_index == from_index => Some(to_index),
            Some(current_index) => {
                let after_remove = if from_index < current_index {
                    current_index - 1
                } else {
                    current_index
                };
                Some(if to_index <= after_remove {
                    after_remove + 1
                } else {
                    after_remove
                })
            }
        }
        .map(|value| {
            u32::try_from(value).map_err(|_| "Playback queue index is out of range.".to_string())
        })
        .transpose()?;

        self.status.current_index = next_current_index;
        self.status.play_next_queue_len = if self.status.current_index == previous_current_index {
            clamped_play_next_queue_len(
                self.status.queue.len(),
                self.status.current_index,
                previous_play_next_queue_len,
            )
        } else {
            adjusted_play_next_queue_len_for_current_change(
                self.status.queue.len(),
                previous_current_index,
                previous_play_next_queue_len,
                self.status.current_index,
            )
        };
        self.status.current_song_id =
            current_song_id(&self.status.queue, self.status.current_index);

        self.sync_queue_state();
        self.persist_state();
        self.report_status();
        Ok(self.state())
    }

    pub fn remove_queue_index(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
        index: u32,
    ) -> Result<PlaybackStatus, String> {
        let remove_index = usize::try_from(index)
            .map_err(|_| "Playback queue index is out of range.".to_string())?;
        if remove_index >= self.status.queue.len() {
            return Err("Playback queue index is out of range.".to_string());
        }

        self.restore_state_deferred = false;
        let current_index = self
            .status
            .current_index
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < self.status.queue.len());
        let removing_current = current_index == Some(remove_index);
        let previous_current_index = self.status.current_index;
        let previous_play_next_queue_len = self.status.play_next_queue_len;
        let was_in_play_next_queue = is_play_next_queue_index(
            self.status.queue.len(),
            previous_current_index,
            previous_play_next_queue_len,
            remove_index,
        );
        let was_loaded = matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        );
        let autoplay = self.should_autoplay();

        if removing_current {
            self.sync_current_position_from_backend()?;
        } else {
            self.sync_current_position_for_queue_mutation("remove_queue_index");
        }
        let removed_position_ms = self.status.current_position_ms;
        let removed_entry = self.status.queue[remove_index].clone();

        self.clear_gapless_preparation();

        if removing_current && was_loaded {
            self.backend.stop()?;
            self.clear_native_events();
            if let Some(context) = context {
                self.report_server_playback_state(
                    context,
                    &removed_entry,
                    removed_position_ms,
                    ServerPlaybackState::Stopped,
                );
            }
            self.clear_server_reporting();
        }

        self.status.queue.remove(remove_index);
        let next_play_next_queue_len = if removing_current && previous_play_next_queue_len > 0 {
            previous_play_next_queue_len.saturating_sub(1)
        } else if was_in_play_next_queue {
            previous_play_next_queue_len.saturating_sub(1)
        } else {
            previous_play_next_queue_len
        };
        let next_current_index = match current_index {
            None => None,
            Some(_) if self.status.queue.is_empty() => None,
            Some(current_index) if current_index > remove_index => Some(current_index - 1),
            Some(current_index) if current_index < remove_index => Some(current_index),
            Some(_) => Some(remove_index.min(self.status.queue.len() - 1)),
        }
        .map(|value| {
            u32::try_from(value).map_err(|_| "Playback queue index is out of range.".to_string())
        })
        .transpose()?;

        self.status.current_index = next_current_index;
        self.status.play_next_queue_len = clamped_play_next_queue_len(
            self.status.queue.len(),
            self.status.current_index,
            next_play_next_queue_len,
        );
        self.status.current_song_id =
            current_song_id(&self.status.queue, self.status.current_index);

        if removing_current {
            self.status.current_position_ms = 0;
            self.status.playback_error = None;
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.interrupted_resume_state = None;

            if self.status.queue.is_empty() {
                self.status.playing_state = PlayingState::Idle;
            } else {
                self.status.playing_state = PlayingState::Stopped;
                if was_loaded {
                    if let (Some(context), Some(next_index)) = (context, self.status.current_index)
                    {
                        self.sync_queue_state();
                        self.load_track_at_index(context, next_index, 0, autoplay)?;
                        self.persist_state();
                        self.process_native_events_inner(Some(context), false)?;
                        self.report_status();
                        return Ok(self.state());
                    }
                }
            }
        }

        self.sync_queue_state();
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
            self.status.playback_error = None;
            self.status.interrupt_reason = None;
            self.sync_actual_stream_info_from_backend();
            self.reset_output_stream_recovery_attempts();
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

        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        ) {
            self.sync_current_position_from_backend()?;
            self.report_current_server_playback_stopped(context, self.status.current_position_ms);
        }
        let previous_current_index = self.status.current_index;
        let previous_play_next_queue_len = self.status.play_next_queue_len;
        self.backend.stop()?;
        self.clear_native_events();
        self.status.current_index = Some(index);
        self.status.play_next_queue_len = adjusted_play_next_queue_len_for_current_change(
            self.status.queue.len(),
            previous_current_index,
            previous_play_next_queue_len,
            self.status.current_index,
        );
        self.status.current_song_id = current_song_id(&self.status.queue, Some(index));
        self.status.current_position_ms = 0;
        self.status.playing_state = PlayingState::Stopped;
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
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
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
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
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
        self.interrupted_resume_state = None;
        self.reset_output_stream_recovery_attempts();
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
                    self.status.playback_error = None;
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
            self.status.playback_error = None;
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
        let previous_play_next_queue_len = self.status.play_next_queue_len;
        self.status.play_next_queue_len = adjusted_play_next_queue_len_for_current_change(
            self.status.queue.len(),
            Some(current_index),
            previous_play_next_queue_len,
            Some(next_index),
        );
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            self.sync_current_position_from_backend()?;
            self.report_current_server_playback_stopped(context, self.status.current_position_ms);
            self.load_track_at_index(context, next_index, 0, autoplay)?;
        } else {
            self.status.current_index = Some(next_index);
            self.status.current_song_id = current_song_id(&self.status.queue, Some(next_index));
            self.status.current_position_ms = 0;
            self.status.playing_state = PlayingState::Stopped;
            self.status.playback_error = None;
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

        // Sync the real position first so the threshold decision below is based on the
        // backend's actual playback position rather than a stale reported value.
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            self.sync_current_position_from_backend()?;
        }

        // Standard "previous" behavior: near the head of the track, jump to the previous
        // track; once playback has progressed past the threshold — or there is no previous
        // track (first entry) — restart the current track from the beginning instead.
        if current_index == 0 || self.status.current_position_ms > PREV_RESTART_THRESHOLD_MS {
            let state = self.seek(context, 0)?;
            self.persist_state();
            return Ok(state);
        }

        let prev_index = current_index - 1;
        let autoplay = self.should_autoplay();
        let previous_play_next_queue_len = self.status.play_next_queue_len;
        self.status.play_next_queue_len = adjusted_play_next_queue_len_for_current_change(
            self.status.queue.len(),
            Some(current_index),
            previous_play_next_queue_len,
            Some(prev_index),
        );
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            self.sync_current_position_from_backend()?;
            self.report_current_server_playback_stopped(context, self.status.current_position_ms);
            self.load_track_at_index(context, prev_index, 0, autoplay)?;
        } else {
            self.status.current_index = Some(prev_index);
            self.status.current_song_id = current_song_id(&self.status.queue, Some(prev_index));
            self.status.current_position_ms = 0;
            self.status.playing_state = PlayingState::Stopped;
            self.status.playback_error = None;
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
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
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
        self.restore_state_deferred = false;
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
        if self.restore_state_deferred {
            return;
        }

        let state_file = PlaybackStateFile {
            version: 1,
            profile_id: profile_id.clone(),
            queue: self.status.queue.clone(),
            current_index: self.status.current_index,
            play_next_queue_len: self.status.play_next_queue_len,
            current_position_ms: self.status.current_position_ms,
        };

        if let Err(error) = self.persister.save(&state_file) {
            log::warn!("controller.persist_state: failed to save: {error}");
        }
    }

    /// Sync position from backend and persist. Used for app exit handler.
    pub fn sync_and_persist(&mut self) {
        if self.active_profile_id.is_some() && !self.restore_state_deferred {
            if matches!(self.status.playing_state, PlayingState::Playing) {
                let _ = self.sync_current_position_from_backend();
            }
            self.persist_state();
        }
        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        ) {
            if let Err(error) = self.backend.stop() {
                log::warn!("controller.sync_and_persist: failed to stop backend: {error}");
            }
            self.clear_native_events();
        }
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
        clamp_play_next_queue_len(&mut self.status);
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
        self.status.playback_error = None;
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
            let playback_error = playback_error_from_load_failure(&error);
            let returned_error = playback_error.message.clone();
            self.status.playing_state = PlayingState::Error;
            self.status.playback_error = Some(playback_error);
            self.status.interrupt_reason = None;
            self.status.pending_seek_position_ms = None;
            self.status.actual_stream_info = None;
            self.clear_server_reporting();
            self.report_status();
            return Err(returned_error);
        }

        self.clear_native_events();
        self.status.playing_state = if autoplay {
            PlayingState::Playing
        } else {
            PlayingState::Paused
        };
        self.status.current_position_ms = requested_position_ms;
        self.status.current_song_id = Some(song_id);
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.replace_actual_stream_info_from_backend();
        self.interrupted_resume_state = None;
        self.reset_output_stream_recovery_attempts();
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
        self.status.playback_error = None;
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
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.interrupted_resume_state = None;
        self.reset_output_stream_recovery_attempts();
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
        let stream_mode = self.stream_mode;
        let transcoding_max_bit_rate = self.transcoding_max_bit_rate;
        let requested_transcoding_codec = self.transcoding_codec;
        let transcoding_codec = match stream_mode {
            PlaybackStreamMode::Raw => requested_transcoding_codec,
            PlaybackStreamMode::Transcoding => {
                self.supported_transcoding_codec(requested_transcoding_codec)
            }
        };
        let stream_max_bit_rate = match stream_mode {
            PlaybackStreamMode::Raw => None,
            PlaybackStreamMode::Transcoding => Some(transcoding_max_bit_rate),
        };
        let transcoding_stream_format = match stream_mode {
            PlaybackStreamMode::Raw => None,
            PlaybackStreamMode::Transcoding => transcoding_codec.stream_format(),
        };

        log::info!(
            "controller.{}: song_id={} path={:?} requested_position_ms={} server_offset_capability={} stream_offset_seconds={:?} stream_mode={:?} stream_max_bit_rate={:?} local_start_position_ms={} autoplay={}",
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
            stream_mode,
            stream_max_bit_rate,
            local_start_position_ms,
            autoplay
        );

        if requested_position_ms == 0 && stream_mode == PlaybackStreamMode::Raw {
            let raw_stream = build_stream_request(
                context.client,
                song_id,
                stream_offset_seconds,
                None,
                Some("raw"),
                self.backend
                    .should_estimate_stream_content_length(stream_mode, true),
            )?;
            let raw_request = self.build_backend_load_request(
                entry,
                raw_stream,
                true,
                None,
                requested_position_ms,
                local_start_position_ms,
                autoplay,
            );
            let raw_info = self.build_stream_info(
                if prepare_only {
                    PlaybackStreamRequestKind::Prepare
                } else {
                    PlaybackStreamRequestKind::Load
                },
                entry,
                stream_mode,
                true,
                Some("raw"),
                None,
                stream_offset_seconds,
                requested_position_ms,
                local_start_position_ms,
                autoplay,
                &raw_request.request,
            );
            return match self.apply_stream_request(raw_info, raw_request, prepare_only) {
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
                    if is_output_device_load_error(&raw_error) {
                        return Err(format!(
                            "Failed to {} playback stream: {raw_error}",
                            if prepare_only { "prepare" } else { "load" }
                        ));
                    }
                    let fallback_stream = build_stream_request(
                        context.client,
                        song_id,
                        stream_offset_seconds,
                        None,
                        None,
                        self.backend
                            .should_estimate_stream_content_length(stream_mode, false),
                    )?;
                    let fallback_request = self.build_backend_load_request(
                        entry,
                        fallback_stream,
                        false,
                        None,
                        requested_position_ms,
                        local_start_position_ms,
                        autoplay,
                    );
                    let fallback_info = self.build_stream_info(
                        if prepare_only {
                            PlaybackStreamRequestKind::Prepare
                        } else {
                            PlaybackStreamRequestKind::Load
                        },
                        entry,
                        stream_mode,
                        false,
                        None,
                        None,
                        stream_offset_seconds,
                        requested_position_ms,
                        local_start_position_ms,
                        autoplay,
                        &fallback_request.request,
                    );
                    match self.apply_stream_request(fallback_info, fallback_request, prepare_only) {
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

        let standard_stream = build_stream_request(
            context.client,
            song_id,
            stream_offset_seconds,
            stream_max_bit_rate,
            transcoding_stream_format,
            self.backend
                .should_estimate_stream_content_length(stream_mode, false),
        )?;
        let standard_request = self.build_backend_load_request(
            entry,
            standard_stream,
            false,
            None,
            requested_position_ms,
            local_start_position_ms,
            autoplay,
        );
        let standard_info = self.build_stream_info(
            if prepare_only {
                PlaybackStreamRequestKind::Prepare
            } else {
                PlaybackStreamRequestKind::Load
            },
            entry,
            stream_mode,
            false,
            transcoding_stream_format,
            stream_max_bit_rate,
            stream_offset_seconds,
            requested_position_ms,
            local_start_position_ms,
            autoplay,
            &standard_request.request,
        );
        self.apply_stream_request(standard_info, standard_request, prepare_only)
            .map_err(|error| {
                format!(
                    "Failed to {} playback stream: {error}",
                    if prepare_only { "prepare" } else { "load" }
                )
            })
    }

    fn supported_transcoding_codec(
        &self,
        requested: PlaybackTranscodingCodec,
    ) -> PlaybackTranscodingCodec {
        if requested == PlaybackTranscodingCodec::Auto
            || self
                .playback_capabilities
                .transcoding_codecs
                .contains(&requested)
        {
            return requested;
        }

        if self
            .playback_capabilities
            .transcoding_codecs
            .contains(&PlaybackTranscodingCodec::Mp3)
        {
            log::warn!(
                "controller: requested transcoding codec {:?} is unsupported by the playback backend; falling back to MP3",
                requested
            );
            return PlaybackTranscodingCodec::Mp3;
        }

        log::warn!(
            "controller: requested transcoding codec {:?} is unsupported by the playback backend; omitting explicit stream format",
            requested
        );
        PlaybackTranscodingCodec::Auto
    }

    fn build_backend_load_request(
        &self,
        entry: &SongResponse,
        request: PreparedBinaryRequest,
        use_source_media_type_hint: bool,
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
            source_content_type: if use_source_media_type_hint {
                entry.content_type.clone()
            } else {
                None
            },
            source_suffix: if use_source_media_type_hint {
                entry.suffix.clone()
            } else {
                None
            },
            artwork_path,
            absolute_start_position_ms,
            local_start_position_ms,
            autoplay,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_stream_info(
        &self,
        request_kind: PlaybackStreamRequestKind,
        entry: &SongResponse,
        stream_mode: PlaybackStreamMode,
        raw_stream: bool,
        stream_format: Option<&str>,
        stream_max_bit_rate: Option<u32>,
        stream_offset_seconds: Option<u32>,
        requested_position_ms: u32,
        local_start_position_ms: u32,
        autoplay: bool,
        request: &PreparedBinaryRequest,
    ) -> PlaybackStreamInfo {
        PlaybackStreamInfo {
            request_kind,
            song_id: entry.id.clone(),
            song_path: entry.path.clone(),
            source_content_type: entry.content_type.clone(),
            source_suffix: entry.suffix.clone(),
            source_bit_rate: entry.bit_rate,
            source_size: entry.size,
            stream_mode,
            raw_stream,
            stream_format: stream_format.map(ToString::to_string),
            stream_max_bit_rate,
            stream_offset_seconds,
            requested_position_ms,
            local_start_position_ms,
            autoplay,
            stream_endpoint: stream_request_endpoint(&request.url),
            stream_url: redacted_stream_url(&request.url),
        }
    }

    fn apply_stream_request(
        &mut self,
        stream_info: PlaybackStreamInfo,
        request: PlaybackBackendLoadRequest,
        prepare_only: bool,
    ) -> Result<Option<u64>, String> {
        self.status.last_stream_request = Some(stream_info.clone());
        let result = if prepare_only {
            self.backend.prepare(request).map(Some)
        } else {
            self.backend.load(request).map(|_| None)
        };

        if result.is_ok() {
            if prepare_only {
                self.status.prepared_stream_info = Some(stream_info);
            } else {
                self.status.active_stream_info = Some(stream_info);
                self.status.prepared_stream_info = None;
            }
        }

        result
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
                let activated = self.try_activate_backend_prepared_track(autoplay)?;
                if activated {
                    let active_stream_info =
                        self.status
                            .prepared_stream_info
                            .take()
                            .map(|mut stream_info| {
                                stream_info.request_kind =
                                    PlaybackStreamRequestKind::ActivatePrepared;
                                stream_info.autoplay = autoplay;
                                stream_info
                            });
                    self.status.active_stream_info = active_stream_info;
                }
                Ok(activated)
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

    fn should_preserve_active_track_for_shared_state(
        &self,
        state: &ConnectPlaybackState,
        is_active_device: bool,
    ) -> bool {
        if !is_active_device {
            return false;
        }
        if self.status.playing_state != state.playing_state {
            return false;
        }
        if !matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            return false;
        }

        let Some(current_entry) =
            current_queue_entry(&self.status.queue, self.status.current_index)
        else {
            return false;
        };
        let Some(next_entry) = current_queue_entry(&state.queue, state.current_index) else {
            return false;
        };

        if state.current_song_id.as_deref() != Some(next_entry.id.as_str()) {
            return false;
        }

        current_entry.id == next_entry.id && current_entry.path == next_entry.path
    }

    fn sync_current_position_for_queue_mutation(&mut self, operation: &str) {
        if let Err(error) = self.sync_current_position_from_backend() {
            log::warn!(
                "controller.{operation}: failed to sync position before queue update: {error}"
            );
        }
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
        self.status.playback_error = None;
        Ok(())
    }

    fn sync_actual_stream_info_from_backend(&mut self) {
        if self.status.current_index.is_none() || self.status.current_song_id.is_none() {
            self.status.actual_stream_info = None;
            return;
        }

        if !matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        ) {
            self.status.actual_stream_info = None;
            return;
        }

        match self.backend.current_stream_info() {
            Ok(Some(info)) => {
                self.status.actual_stream_info = Some(info);
            }
            Ok(None) => {
                // Backends may briefly have no selected track metadata during native
                // transitions. Keep the last known stream info until a new value arrives
                // or the queue/current track is explicitly cleared.
            }
            Err(error) => {
                log::warn!(
                    "controller.sync_actual_stream_info_from_backend: failed to sync stream info: {error}"
                );
            }
        }
    }

    fn replace_actual_stream_info_from_backend(&mut self) {
        match self.backend.current_stream_info() {
            Ok(info) => {
                self.status.actual_stream_info = info;
            }
            Err(error) => {
                log::warn!(
                    "controller.sync_actual_stream_info_from_backend: failed to sync stream info: {error}"
                );
            }
        }
    }

    fn process_native_events_inner(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
        report_changes: bool,
    ) -> Result<bool, String> {
        let events = self.native_events.drain_events();

        let mut changed = false;
        for event in events {
            changed |= self.apply_native_event(event, context)?;
        }
        changed |= self.process_due_output_stream_recovery(context)?;

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
                self.status.playback_error = None;
                Ok(true)
            }
            PlaybackNativeEvent::Ready { position_ms } => {
                self.status.current_position_ms = position_ms;
                self.status.playback_error = None;
                self.confirm_pending_seek();
                self.status.interrupt_reason = None;
                if matches!(self.status.playing_state, PlayingState::Interrupted) {
                    self.status.playing_state = self
                        .interrupted_resume_state
                        .clone()
                        .unwrap_or(PlayingState::Paused);
                }
                self.interrupted_resume_state = None;
                self.sync_actual_stream_info_from_backend();
                Ok(true)
            }
            PlaybackNativeEvent::Playing { position_ms } => {
                self.status.playing_state = PlayingState::Playing;
                self.status.interrupt_reason = None;
                self.status.current_position_ms = position_ms;
                self.status.playback_error = None;
                self.confirm_pending_seek();
                self.interrupted_resume_state = None;
                self.sync_actual_stream_info_from_backend();
                Ok(true)
            }
            PlaybackNativeEvent::Paused { position_ms } => {
                self.status.playing_state = PlayingState::Paused;
                self.status.interrupt_reason = None;
                self.status.current_position_ms = position_ms;
                self.status.playback_error = None;
                self.confirm_pending_seek();
                self.interrupted_resume_state = None;
                self.sync_actual_stream_info_from_backend();
                Ok(true)
            }
            PlaybackNativeEvent::SeekProcessed { position_ms } => {
                self.status.current_position_ms = position_ms;
                self.status.playback_error = None;
                self.confirm_pending_seek();
                Ok(true)
            }
            PlaybackNativeEvent::Error { message, handled } => {
                if !handled && is_output_stream_runtime_error(&message) {
                    return self.handle_output_stream_error(message, context);
                }
                self.transition_to_playback_error(message, handled);
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

    fn handle_output_stream_error(
        &mut self,
        message: String,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<bool, String> {
        if self.pending_output_stream_recovery.is_some() {
            return Ok(false);
        }

        if !matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused | PlayingState::Interrupted
        ) {
            self.transition_to_playback_error(message, false);
            return Ok(true);
        }

        if self.output_stream_recovery_attempts >= OUTPUT_STREAM_RECOVERY_MAX_ATTEMPTS {
            let current_position_ms = self.status.current_position_ms;
            let stop_error = self.stop_backend_after_output_stream_error();
            if let Some(context) = context {
                self.report_current_server_playback_stopped(context, current_position_ms);
            }
            let message = match stop_error {
                Some(stop_error) => format!(
                    "{message}; automatic output stream recovery limit reached; failed to stop output stream: {stop_error}"
                ),
                None => format!("{message}; automatic output stream recovery limit reached"),
            };
            self.transition_to_playback_error(message, false);
            return Ok(true);
        }

        let Some(context) = context else {
            let stop_error = self.stop_backend_after_output_stream_error();
            let message = match stop_error {
                Some(stop_error) => {
                    format!("{message}; cannot recover output stream without an active session; failed to stop output stream: {stop_error}")
                }
                None => {
                    format!("{message}; cannot recover output stream without an active session")
                }
            };
            self.transition_to_playback_error(message, false);
            return Ok(true);
        };

        let Some(index) = self.status.current_index else {
            let current_position_ms = self.status.current_position_ms;
            let stop_error = self.stop_backend_after_output_stream_error();
            self.report_current_server_playback_stopped(context, current_position_ms);
            let message = match stop_error {
                Some(stop_error) => {
                    format!("{message}; cannot recover output stream without a current queue index; failed to stop output stream: {stop_error}")
                }
                None => {
                    format!("{message}; cannot recover output stream without a current queue index")
                }
            };
            self.transition_to_playback_error(message, false);
            return Ok(true);
        };

        if matches!(
            self.status.playing_state,
            PlayingState::Playing | PlayingState::Paused
        ) {
            if let Err(error) = self.sync_current_position_from_backend() {
                log::warn!(
                    "controller.handle_output_stream_error: failed to sync position before recovery: {error}"
                );
            }
        }

        let requested_position_ms = self.status.current_position_ms;
        let autoplay = self.should_autoplay();
        let next_attempt = self.output_stream_recovery_attempts.saturating_add(1);

        if let Some(stop_error) = self.stop_backend_after_output_stream_error() {
            self.report_current_server_playback_stopped(context, requested_position_ms);
            self.transition_to_playback_error(
                format!("{message}; failed to stop output stream for recovery: {stop_error}"),
                false,
            );
            return Ok(true);
        }

        log::warn!(
            "controller.handle_output_stream_error: scheduling output stream recovery attempt={next_attempt} index={index} position_ms={requested_position_ms}"
        );

        self.schedule_output_stream_recovery(
            message,
            index,
            requested_position_ms,
            autoplay,
            next_attempt,
        );
        Ok(true)
    }

    fn process_due_output_stream_recovery(
        &mut self,
        context: Option<&PlaybackRuntimeContext<'_>>,
    ) -> Result<bool, String> {
        let Some(plan) = self.pending_output_stream_recovery.clone() else {
            return Ok(false);
        };

        let now = Instant::now();
        if now < plan.not_before {
            return Ok(false);
        }

        let Some(context) = context else {
            self.pending_output_stream_recovery = None;
            self.output_stream_recovery_attempts = OUTPUT_STREAM_RECOVERY_MAX_ATTEMPTS;
            self.transition_to_playback_error(
                format!(
                    "{}; cannot recover output stream without an active session",
                    plan.message
                ),
                false,
            );
            return Ok(true);
        };

        self.pending_output_stream_recovery = None;
        self.run_output_stream_recovery_attempt(plan, context)
    }

    fn run_output_stream_recovery_attempt(
        &mut self,
        plan: OutputStreamRecoveryPlan,
        context: &PlaybackRuntimeContext<'_>,
    ) -> Result<bool, String> {
        log::warn!(
            "controller.run_output_stream_recovery_attempt: attempt={} index={} position_ms={}",
            plan.attempt,
            plan.index,
            plan.requested_position_ms
        );

        match self.reload_track_at_index_exact(
            context,
            plan.index,
            plan.requested_position_ms,
            plan.autoplay,
        ) {
            Ok(()) => {
                self.output_stream_recovery_attempts = plan.attempt;
                Ok(true)
            }
            Err(reload_error) if plan.attempt < OUTPUT_STREAM_RECOVERY_MAX_ATTEMPTS => {
                self.output_stream_recovery_attempts = plan.attempt;
                let next_attempt = plan.attempt.saturating_add(1);
                log::warn!(
                    "controller.run_output_stream_recovery_attempt: attempt={} failed; scheduling attempt={next_attempt}: {reload_error}",
                    plan.attempt
                );
                self.schedule_output_stream_recovery(
                    plan.message,
                    plan.index,
                    plan.requested_position_ms,
                    plan.autoplay,
                    next_attempt,
                );
                Ok(true)
            }
            Err(reload_error) => {
                self.output_stream_recovery_attempts = OUTPUT_STREAM_RECOVERY_MAX_ATTEMPTS;
                self.report_current_server_playback_stopped(context, plan.requested_position_ms);
                let playback_error = playback_error_from_load_failure(&format!(
                    "{}; automatic output stream recovery failed after {} attempts: {reload_error}",
                    plan.message, plan.attempt
                ));
                self.status.playing_state = PlayingState::Error;
                self.status.current_position_ms = plan.requested_position_ms;
                self.status.playback_error = Some(playback_error);
                self.status.interrupt_reason = None;
                self.status.pending_seek_position_ms = None;
                self.status.actual_stream_info = None;
                self.interrupted_resume_state = None;
                self.clear_gapless_preparation();
                self.clear_server_reporting();
                Ok(true)
            }
        }
    }

    fn schedule_output_stream_recovery(
        &mut self,
        message: String,
        index: u32,
        requested_position_ms: u32,
        autoplay: bool,
        attempt: u8,
    ) {
        let delay = output_stream_recovery_delay(attempt);
        self.pending_output_stream_recovery = Some(OutputStreamRecoveryPlan {
            message,
            index,
            requested_position_ms,
            autoplay,
            attempt,
            not_before: Instant::now() + delay,
        });
        self.status.playing_state = PlayingState::Interrupted;
        self.status.interrupt_reason = Some(InterruptReason::FullReload);
        self.status.pending_seek_position_ms = Some(requested_position_ms);
        self.status.current_position_ms = requested_position_ms;
        self.status.actual_stream_info = None;
        self.status.playback_error = None;
        self.interrupted_resume_state = None;
        crate::playback::spawn_controller_process_native_events_after(delay);
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
        let Some(entry) =
            current_queue_entry(&self.status.queue, Some(target.queue_index)).cloned()
        else {
            self.clear_gapless_preparation();
            return Ok(false);
        };

        let ended_entry =
            current_queue_entry(&self.status.queue, self.status.current_index).cloned();
        let ended_position_ms = self.status.current_position_ms;
        if let (Some(context), Some(entry)) = (context, ended_entry.as_ref()) {
            self.report_server_playback_state(
                context,
                entry,
                ended_position_ms,
                ServerPlaybackState::Stopped,
            );
        }

        self.status.play_next_queue_len = adjusted_play_next_queue_len_for_current_change(
            self.status.queue.len(),
            self.status.current_index,
            self.status.play_next_queue_len,
            Some(target.queue_index),
        );
        self.status.playing_state = PlayingState::Playing;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.current_index = Some(target.queue_index);
        self.status.current_song_id = Some(entry.id.clone());
        self.status.current_position_ms = 0;
        self.status.playback_error = None;
        self.status.active_stream_info =
            self.status
                .prepared_stream_info
                .take()
                .map(|mut stream_info| {
                    stream_info.request_kind = PlaybackStreamRequestKind::ActivatePrepared;
                    stream_info.autoplay = true;
                    stream_info
                });
        self.sync_actual_stream_info_from_backend();
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
            if let (Some(context), Some(entry)) = (context, ended_entry.as_ref()) {
                self.report_server_playback_state(
                    context,
                    entry,
                    ended_position_ms,
                    ServerPlaybackState::Stopped,
                );
            }
            self.transition_to_stopped_end_of_queue();
            return Ok(true);
        };
        let has_next = usize::try_from(next_index)
            .ok()
            .is_some_and(|index| index < self.status.queue.len());
        if !has_next {
            if let (Some(context), Some(entry)) = (context, ended_entry.as_ref()) {
                self.report_server_playback_state(
                    context,
                    entry,
                    ended_position_ms,
                    ServerPlaybackState::Stopped,
                );
            }
            self.transition_to_stopped_end_of_queue();
            return Ok(true);
        }

        let Some(runtime_context) = context else {
            self.transition_to_stopped_end_of_queue();
            return Ok(true);
        };
        if let Some(entry) = ended_entry.as_ref() {
            self.report_server_playback_state(
                runtime_context,
                entry,
                ended_position_ms,
                ServerPlaybackState::Stopped,
            );
        }
        self.status.play_next_queue_len = adjusted_play_next_queue_len_for_current_change(
            self.status.queue.len(),
            Some(current_index),
            self.status.play_next_queue_len,
            Some(next_index),
        );
        self.load_track_at_index(runtime_context, next_index, 0, true)?;
        self.report_status();
        Ok(false)
    }

    fn transition_to_stopped_end_of_queue(&mut self) {
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.status.playing_state = PlayingState::Stopped;
        self.status.current_position_ms = 0;
        self.status.playback_error = None;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.actual_stream_info = None;
        self.interrupted_resume_state = None;
    }

    fn clear_native_events(&mut self) {
        let _ = self.native_events.drain_events();
    }

    fn stop_backend_after_output_stream_error(&mut self) -> Option<String> {
        let stop_error = self.backend.stop().err();
        self.clear_native_events();
        self.clear_gapless_preparation();
        self.clear_server_reporting();
        self.status.actual_stream_info = None;
        stop_error
    }

    fn transition_to_playback_error(&mut self, message: String, handled: bool) {
        self.status.playing_state = PlayingState::Error;
        self.status.interrupt_reason = None;
        self.status.pending_seek_position_ms = None;
        self.status.playback_error = Some(PlaybackError { message, handled });
        self.status.actual_stream_info = None;
        self.interrupted_resume_state = None;
    }

    fn reset_output_stream_recovery_attempts(&mut self) {
        self.output_stream_recovery_attempts = 0;
        self.pending_output_stream_recovery = None;
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

    fn report_current_server_playback_stopped(
        &mut self,
        context: &PlaybackRuntimeContext<'_>,
        position_ms: u32,
    ) {
        if let Some(entry) =
            current_queue_entry(&self.status.queue, self.status.current_index).cloned()
        {
            self.report_server_playback_state(
                context,
                &entry,
                position_ms,
                ServerPlaybackState::Stopped,
            );
        }
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
                queue_index: self.status.current_index,
                duration_ms: entry.duration.map(|duration| u64::from(duration) * 1000),
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
    max_bit_rate: Option<u32>,
    stream_format: Option<&str>,
    estimate_content_length: bool,
) -> Result<PreparedBinaryRequest, String> {
    client
        .stream(StreamRequest {
            id: song_id.to_string(),
            max_bit_rate,
            format: stream_format.map(ToString::to_string),
            time_offset: stream_offset_seconds,
            size: None,
            estimate_content_length: estimate_content_length.then_some(true),
            converted: None,
        })
        .map_err(format_api_error)
}

fn stream_request_endpoint(url: &reqwest::Url) -> String {
    url.path()
        .trim_start_matches('/')
        .trim_end_matches(".view")
        .to_string()
}

fn redacted_stream_url(url: &reqwest::Url) -> String {
    const SENSITIVE_QUERY_KEYS: &[&str] = &["u", "p", "t", "s", "apiKey", "api_key"];

    let mut redacted = url.clone();
    let query_pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();

    redacted.set_query(None);
    {
        let mut serializer = redacted.query_pairs_mut();
        for (key, value) in query_pairs {
            let redacted_value = if SENSITIVE_QUERY_KEYS
                .iter()
                .any(|sensitive| sensitive.eq_ignore_ascii_case(&key))
            {
                "[redacted]"
            } else {
                value.as_str()
            };
            serializer.append_pair(&key, redacted_value);
        }
    }
    redacted.to_string()
}

fn is_missing_prepared_stream_error(error: &str) -> bool {
    error.to_ascii_lowercase().contains("no prepared stream")
}

fn format_output_device_change_error(error: String, rollback_error: Option<String>) -> String {
    match rollback_error {
        Some(rollback_error) => {
            format!("{error}; failed to roll back output device selection: {rollback_error}")
        }
        None => error,
    }
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

// Play-next region math. This local reducer is intentionally separate from the
// Go Connect reducer (server/internal/realtime/hub.go); the one invariant both
// must preserve (what play_next_queue_len means) is documented in
// docs/playback-queue-semantics.md.
fn play_next_start_index(queue_len: usize, current_index: Option<u32>) -> usize {
    let Some(current_index) = current_index.and_then(|value| usize::try_from(value).ok()) else {
        return 0;
    };
    current_index.saturating_add(1).min(queue_len)
}

fn clamped_play_next_queue_len(
    queue_len: usize,
    current_index: Option<u32>,
    play_next_queue_len: u32,
) -> u32 {
    let start = play_next_start_index(queue_len, current_index);
    let max_len = queue_len.saturating_sub(start);
    play_next_queue_len.min(u32::try_from(max_len).unwrap_or(u32::MAX))
}

fn play_next_end_index(status: &PlaybackStatus) -> usize {
    let start = play_next_start_index(status.queue.len(), status.current_index);
    start
        .saturating_add(usize::try_from(status.play_next_queue_len).unwrap_or(usize::MAX))
        .min(status.queue.len())
}

fn clamp_play_next_queue_len(status: &mut PlaybackStatus) {
    status.play_next_queue_len = clamped_play_next_queue_len(
        status.queue.len(),
        status.current_index,
        status.play_next_queue_len,
    );
}

fn is_play_next_queue_index(
    queue_len: usize,
    current_index: Option<u32>,
    play_next_queue_len: u32,
    queue_index: usize,
) -> bool {
    let start = play_next_start_index(queue_len, current_index);
    let end = start
        .saturating_add(usize::try_from(play_next_queue_len).unwrap_or(usize::MAX))
        .min(queue_len);
    queue_index >= start && queue_index < end
}

fn adjusted_play_next_queue_len_for_current_change(
    queue_len: usize,
    previous_current_index: Option<u32>,
    previous_play_next_queue_len: u32,
    next_current_index: Option<u32>,
) -> u32 {
    if previous_play_next_queue_len == 0 {
        return 0;
    }

    if previous_current_index == next_current_index {
        return clamped_play_next_queue_len(
            queue_len,
            next_current_index,
            previous_play_next_queue_len,
        );
    }

    let previous_start = play_next_start_index(queue_len, previous_current_index);
    let previous_end = previous_start
        .saturating_add(usize::try_from(previous_play_next_queue_len).unwrap_or(usize::MAX))
        .min(queue_len);
    let Some(next_index) = next_current_index.and_then(|value| usize::try_from(value).ok()) else {
        return 0;
    };
    if next_index < previous_start || next_index >= previous_end {
        return 0;
    }

    u32::try_from(previous_end.saturating_sub(next_index).saturating_sub(1)).unwrap_or(u32::MAX)
}

fn display_cover_art_id(entry: &SongResponse) -> Option<&str> {
    entry
        .display_cover_art_id
        .as_deref()
        .and_then(trimmed_text)
        .or_else(|| entry.cover_art_id.as_deref().and_then(trimmed_text))
}

fn trimmed_text(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
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
        current_song_id, playback_error_from_load_failure, PlaybackController,
        PlaybackRuntimeContext, PlaybackStatus, HANDLED_PLAYBACK_ERROR_PREFIX,
        PREV_RESTART_THRESHOLD_MS,
    };
    use crate::{
        models::{
            CapabilityMatrix, ConnectPlaybackState, GaplessState, InterruptReason,
            PlaybackActualStreamInfo, PlaybackCapabilities, PlaybackStreamMode,
            PlaybackStreamRequestKind, PlaybackTranscodingCodec, PlayingState, SongResponse,
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
        playback_state::{NoopPlaybackStatePersister, PlaybackStateFile, PlaybackStatePersister},
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
        load_artwork_paths: Vec<Option<String>>,
        load_source_content_types: Vec<Option<String>>,
        load_source_suffixes: Vec<Option<String>>,
        prepare_calls: Vec<String>,
        prepare_artwork_paths: Vec<Option<String>>,
        artwork_update_calls: Vec<(String, Option<String>)>,
        next_prepare_generation: u64,
        seek_calls_ms: Vec<u32>,
        current_position_ms: u32,
        current_position_calls: usize,
        fail_first_load: bool,
        always_fail_load: bool,
        load_error: Option<String>,
        fail_standard_load: bool,
        seek_behavior: MockSeekBehavior,
        has_failed_first_load: bool,
        pause_calls: usize,
        resume_calls: usize,
        stop_calls: usize,
        volume_calls: Vec<f32>,
        output_device_calls: Vec<Option<String>>,
        fail_output_device: bool,
        activate_prepared_calls: usize,
        clear_prepared_calls: usize,
        activated_prepared_urls: Vec<String>,
        current_stream_info: Option<PlaybackActualStreamInfo>,
        prepared_request: Option<MockPreparedRequest>,
    }

    #[derive(Debug, Clone)]
    struct MockPreparedRequest {
        url: String,
        absolute_start_position_ms: u32,
        actual_stream_info: PlaybackActualStreamInfo,
    }

    #[derive(Debug, Default)]
    struct MockReporterState {
        reported_states: Vec<PlaybackStatus>,
    }

    #[derive(Debug, Default)]
    struct MockQueueSyncState {
        synced_states: Vec<PlaybackStatus>,
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
        estimate_stream_content_length: bool,
        native_events: Option<Arc<Mutex<Vec<PlaybackNativeEvent>>>>,
    }

    struct MockReporter {
        state: Arc<Mutex<MockReporterState>>,
    }

    struct MockServerReporter {
        state: Arc<Mutex<MockServerReporterState>>,
    }

    struct MockQueueSync {
        state: Arc<Mutex<MockQueueSyncState>>,
    }

    #[derive(Clone, Default)]
    struct RecordingPersister {
        states: Arc<Mutex<Vec<PlaybackStateFile>>>,
    }

    #[derive(Default)]
    struct MockNativeEventSource {
        events: Arc<Mutex<Vec<PlaybackNativeEvent>>>,
    }

    fn mock_actual_stream_info(codec: &str) -> PlaybackActualStreamInfo {
        PlaybackActualStreamInfo {
            codec: Some(codec.to_string()),
            codec_profile: None,
            sample_rate: Some(44_100),
            channels: Some(2),
            bit_depth: Some(16),
            bitrate: Some(320_000),
            sample_format: Some("f32".to_string()),
            mime_type: Some("audio/mock".to_string()),
        }
    }

    fn mock_actual_stream_info_for_url(url: &str) -> PlaybackActualStreamInfo {
        if url.contains("song-b") {
            mock_actual_stream_info("song-b")
        } else if url.contains("song-c") {
            mock_actual_stream_info("song-c")
        } else {
            mock_actual_stream_info("song-a")
        }
    }

    impl PlaybackBackend for MockBackend {
        fn capabilities(&self) -> PlaybackCapabilities {
            self.playback_capabilities.clone()
        }

        fn should_estimate_stream_content_length(
            &self,
            _stream_mode: PlaybackStreamMode,
            _raw_stream: bool,
        ) -> bool {
            self.estimate_stream_content_length
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
            state.load_artwork_paths.push(request.artwork_path.clone());
            state
                .load_source_content_types
                .push(request.source_content_type.clone());
            state
                .load_source_suffixes
                .push(request.source_suffix.clone());
            if state.always_fail_load {
                return Err("stream rejected".to_string());
            }
            if let Some(error) = state.load_error.clone() {
                return Err(error);
            }
            if state.fail_standard_load && !request.request.url.as_str().contains("format=raw") {
                return Err("standard stream rejected".to_string());
            }
            if state.fail_first_load && !state.has_failed_first_load {
                state.has_failed_first_load = true;
                return Err("raw stream rejected".to_string());
            }

            state.current_position_ms = request.absolute_start_position_ms;
            state.current_stream_info = Some(mock_actual_stream_info_for_url(
                request.request.url.as_str(),
            ));

            Ok(())
        }

        fn prepare(&mut self, request: PlaybackBackendLoadRequest) -> Result<u64, String> {
            let mut state = self.state.lock().unwrap();
            let url = request.request.url.to_string();
            state.prepare_calls.push(url.clone());
            state
                .prepare_artwork_paths
                .push(request.artwork_path.clone());
            if state.always_fail_load {
                return Err("stream rejected".to_string());
            }
            if let Some(error) = state.load_error.clone() {
                return Err(error);
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
                actual_stream_info: mock_actual_stream_info_for_url(&url),
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
            state.current_stream_info = Some(request.actual_stream_info);
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

        fn current_stream_info(&self) -> Result<Option<PlaybackActualStreamInfo>, String> {
            Ok(self.state.lock().unwrap().current_stream_info.clone())
        }

        fn set_volume(&mut self, volume: f32) -> Result<(), String> {
            self.state.lock().unwrap().volume_calls.push(volume);
            Ok(())
        }

        fn set_output_device(&mut self, output_device_id: Option<String>) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            if state.fail_output_device {
                return Err("output device rejected".to_string());
            }
            state.output_device_calls.push(output_device_id);
            Ok(())
        }

        fn update_artwork(
            &mut self,
            media_id: &str,
            artwork_path: Option<String>,
        ) -> Result<(), String> {
            self.state
                .lock()
                .unwrap()
                .artwork_update_calls
                .push((media_id.to_string(), artwork_path));
            Ok(())
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

    impl PlaybackStatePersister for RecordingPersister {
        fn save(&self, state: &PlaybackStateFile) -> Result<(), String> {
            self.states.lock().unwrap().push(state.clone());
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

    impl QueueSyncGateway for MockQueueSync {
        fn sync_queue(&mut self, status: &PlaybackStatus) -> Result<(), String> {
            self.state
                .lock()
                .unwrap()
                .synced_states
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

    fn mock_playback_capabilities(gapless_playback: bool) -> PlaybackCapabilities {
        mock_playback_capabilities_with_codecs(
            gapless_playback,
            vec![PlaybackTranscodingCodec::Mp3],
        )
    }

    fn mock_playback_capabilities_with_codecs(
        gapless_playback: bool,
        transcoding_codecs: Vec<PlaybackTranscodingCodec>,
    ) -> PlaybackCapabilities {
        PlaybackCapabilities {
            gapless_playback,
            transcoding_codecs,
            output_device_selection: false,
        }
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
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length: true,
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
            1.0,
            None,
        );
        (controller, state, reporter_state)
    }

    fn controller_with_initial_output_device(
        initial_output_device_id: Option<String>,
        fail_output_device: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState {
            fail_output_device,
            ..MockBackendState::default()
        }));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length: true,
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
            1.0,
            initial_output_device_id,
        );
        (controller, state, reporter_state)
    }

    fn controller_with_mock_backend_and_persister(
        persister: Box<dyn PlaybackStatePersister>,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length: true,
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
            persister,
            false,
            1.0,
            None,
        );
        (controller, state, reporter_state)
    }

    fn controller_with_mock_backend_and_queue_sync() -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockQueueSyncState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let queue_sync_state = Arc::new(Mutex::new(MockQueueSyncState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length: true,
            native_events: None,
        });
        let reporter: Box<dyn PlaybackReporter> = Box::new(MockReporter {
            state: reporter_state,
        });
        let server_reporter: Box<dyn PlaybackServerReporter> = Box::new(NoopPlaybackServerReporter);
        let queue_sync: Box<dyn QueueSyncGateway> = Box::new(MockQueueSync {
            state: queue_sync_state.clone(),
        });
        let controller = PlaybackController::new(
            backend,
            reporter,
            server_reporter,
            queue_sync,
            Box::new(NoopNativePlaybackEventSource),
            Box::new(NoopPlaybackStatePersister),
            false,
            1.0,
            None,
        );
        (controller, state, queue_sync_state)
    }

    fn controller_with_mock_backend_estimate_content_length(
        estimate_stream_content_length: bool,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length,
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
            1.0,
            None,
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
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length: true,
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
            1.0,
            None,
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
        controller_with_mock_backend_playback_capabilities(mock_playback_capabilities(
            gapless_playback,
        ))
    }

    fn controller_with_mock_backend_playback_capabilities(
        playback_capabilities: PlaybackCapabilities,
    ) -> (
        PlaybackController,
        Arc<Mutex<MockBackendState>>,
        Arc<Mutex<MockReporterState>>,
    ) {
        let state = Arc::new(Mutex::new(MockBackendState::default()));
        let reporter_state = Arc::new(Mutex::new(MockReporterState::default()));
        let backend = Box::new(MockBackend {
            state: state.clone(),
            playback_capabilities,
            estimate_stream_content_length: true,
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
            1.0,
            None,
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
            playback_capabilities: mock_playback_capabilities(true),
            estimate_stream_content_length: true,
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
            1.0,
            None,
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
            })
            .collect()
    }

    #[test]
    fn playback_load_does_not_resolve_cover_art_on_critical_path() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let mut queue = queue_entries(&["song-a"]);
        queue[0].cover_art_id = Some("cover-1".to_string());
        controller.set_queue(queue, Some(0)).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let cover_art_cache =
            crate::cover_art_cache::CoverArtCache::new(temp_dir.path().to_path_buf());
        let mut server = mockito::Server::new();
        let no_cover_art_fetch = server
            .mock("GET", "/rest/getCoverArt.view")
            .expect(0)
            .create();
        let client = opensubsonic_client::OpenSubsonicClient::new(ClientConfig::new(
            normalize_base_url(&server.url()).unwrap(),
            Auth::Token {
                username: "demo".to_string(),
                password: "secret".to_string(),
            },
            "transonic",
        ))
        .unwrap();
        let capability_matrix = CapabilityMatrix::empty();
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: Some(&cover_art_cache),
            profile_id: Some("profile-1"),
        };

        let status = controller.play(&runtime_context).unwrap();

        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(backend_state.lock().unwrap().load_artwork_paths, vec![None]);
        no_cover_art_fetch.assert();
    }

    #[test]
    fn current_artwork_update_target_prefers_display_cover_art_id() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let mut queue = queue_entries(&["song-a"]);
        queue[0].cover_art_id = Some("song-cover".to_string());
        queue[0].display_cover_art_id = Some(" album-cover ".to_string());
        controller.set_queue(queue, Some(0)).unwrap();

        let target = controller.current_artwork_update_target().unwrap();

        assert_eq!(target.queue_index, 0);
        assert_eq!(target.song_id, "song-a");
        assert_eq!(target.cover_art_id, "album-cover");
    }

    #[test]
    fn update_artwork_if_current_updates_backend_for_current_target() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let mut queue = queue_entries(&["song-a"]);
        queue[0].cover_art_id = Some("cover-1".to_string());
        controller.set_queue(queue, Some(0)).unwrap();
        let target = controller.current_artwork_update_target().unwrap();

        let updated = controller
            .update_artwork_if_current(&target, "cache/cover.png".to_string())
            .unwrap();

        assert!(updated);
        assert_eq!(
            backend_state.lock().unwrap().artwork_update_calls,
            vec![("song-a".to_string(), Some("cache/cover.png".to_string()))]
        );
    }

    #[test]
    fn update_artwork_if_current_ignores_stale_target() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let mut queue = queue_entries(&["song-a", "song-b"]);
        queue[0].cover_art_id = Some("cover-a".to_string());
        queue[1].cover_art_id = Some("cover-b".to_string());
        controller.set_queue(queue.clone(), Some(0)).unwrap();
        let stale_target = controller.current_artwork_update_target().unwrap();
        controller.set_queue(queue, Some(1)).unwrap();

        let updated = controller
            .update_artwork_if_current(&stale_target, "cache/cover-a.png".to_string())
            .unwrap();

        assert!(!updated);
        assert!(backend_state
            .lock()
            .unwrap()
            .artwork_update_calls
            .is_empty());
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

        // `prev` at the first entry no longer errors; it restarts the current track from the
        // beginning (there is no previous track to move to).
        let prev_result = controller.prev(&runtime_context).unwrap();
        assert_eq!(prev_result.current_index, Some(0));
        assert_eq!(prev_result.current_position_ms, 0);

        controller.next(&runtime_context).unwrap();
        let next_result = controller.next(&runtime_context);
        assert!(next_result.is_err());
    }

    #[test]
    fn prev_restarts_current_track_past_threshold_and_steps_back_near_head() {
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
        controller.next(&runtime_context).unwrap();
        assert_eq!(controller.state().current_index, Some(1));

        // Past the threshold: `prev` restarts the current track (index unchanged, seek to 0).
        backend_state.lock().unwrap().current_position_ms = PREV_RESTART_THRESHOLD_MS + 2_000;
        let restarted = controller.prev(&runtime_context).unwrap();
        assert_eq!(restarted.current_index, Some(1));
        assert_eq!(restarted.pending_seek_position_ms, Some(0));

        // Near the head: `prev` steps back to the previous track.
        backend_state.lock().unwrap().current_position_ms = PREV_RESTART_THRESHOLD_MS - 2_000;
        let stepped = controller.prev(&runtime_context).unwrap();
        assert_eq!(stepped.current_index, Some(0));
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
    fn server_reporting_stops_previous_track_before_manual_next() {
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
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();
        backend_state.lock().unwrap().current_position_ms = 120_000;
        controller.next(&runtime_context).unwrap();

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
                    position_ms: 120_000,
                    state: ServerPlaybackState::Stopped,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-b".to_string(),
                    position_ms: 0,
                    state: ServerPlaybackState::Starting,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-b".to_string(),
                    position_ms: 0,
                    state: ServerPlaybackState::Playing,
                },
            ]
        );
    }

    #[test]
    fn server_reporting_stops_previous_track_before_auto_advance() {
        let (mut controller, _, server_reporter_state) =
            controller_with_mock_backend_and_server_reporter(true);
        let (client, capability_matrix) = runtime_context_with_reporting(true, true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: Some("profile-1"),
        };
        controller
            .set_queue(queue_entries(&["song-a", "song-b"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();
        controller
            .handle_track_ended(Some(&runtime_context))
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
                    position_ms: 0,
                    state: ServerPlaybackState::Stopped,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-b".to_string(),
                    position_ms: 0,
                    state: ServerPlaybackState::Starting,
                },
                MockServerReportEvent {
                    profile_id: "profile-1".to_string(),
                    song_id: "song-b".to_string(),
                    position_ms: 0,
                    state: ServerPlaybackState::Playing,
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
        assert!(paused.actual_stream_info.is_none());
        backend_state.lock().unwrap().current_stream_info =
            Some(mock_actual_stream_info("song-a-resumed"));

        let resumed = controller.play(&runtime_context).unwrap();
        assert_eq!(resumed.playing_state, PlayingState::Playing);
        assert_eq!(
            resumed
                .actual_stream_info
                .as_ref()
                .and_then(|info| info.codec.as_deref()),
            Some("song-a-resumed")
        );
    }

    #[test]
    fn stop_and_state_sync_clear_actual_stream_info_for_current_track() {
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

        let stopped = controller.stop().unwrap();
        assert_eq!(stopped.playing_state, PlayingState::Stopped);
        assert!(stopped.actual_stream_info.is_none());

        let synced = controller.synced_state().unwrap();
        assert_eq!(synced.playing_state, PlayingState::Stopped);
        assert!(synced.actual_stream_info.is_none());
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
    fn stream_setting_change_skips_backend_clear_without_prepared_gapless_track() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);

        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Mp3,
                None,
            )
            .unwrap();

        assert_eq!(backend_state.lock().unwrap().clear_prepared_calls, 0);
    }

    #[test]
    fn stream_setting_change_clears_existing_prepared_gapless_track() {
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

        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Mp3,
                Some(&runtime_context),
            )
            .unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(backend_state.clear_prepared_calls, 1);
        assert_eq!(backend_state.prepare_calls.len(), 2);
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
        backend_state.lock().unwrap().current_stream_info = None;
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::GaplessTransition);

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        {
            let backend_state = backend_state.lock().unwrap();
            assert_eq!(backend_state.load_calls.len(), 1);
            assert_eq!(backend_state.activate_prepared_calls, 0);
            assert_eq!(backend_state.prepare_calls.len(), 2);
        }

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id.as_deref(), Some("song-b"));
        assert_eq!(
            status
                .active_stream_info
                .as_ref()
                .map(|info| (info.song_id.as_str(), &info.request_kind)),
            Some(("song-b", &PlaybackStreamRequestKind::ActivatePrepared))
        );
        assert_eq!(
            status
                .actual_stream_info
                .as_ref()
                .and_then(|info| info.codec.as_deref()),
            Some("song-a")
        );
        backend_state.lock().unwrap().current_stream_info = Some(mock_actual_stream_info("song-b"));
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Playing { position_ms: 0 });
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(
            status
                .actual_stream_info
                .as_ref()
                .and_then(|info| info.codec.as_deref()),
            Some("song-b")
        );
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
        let mut queue = queue_entries(&["song-a"]);
        queue[0].content_type = Some("audio/x-m4a".to_string());
        queue[0].suffix = Some("m4a".to_string());
        controller.set_queue(queue, Some(0)).unwrap();

        controller.play(&runtime_context).unwrap();

        let state = backend_state.lock().unwrap();
        assert_eq!(state.load_calls.len(), 2);
        assert!(state.load_calls[0].contains("format=raw"));
        assert!(!state.load_calls[1].contains("format=raw"));
        assert_eq!(
            state.load_source_content_types,
            vec![Some("audio/x-m4a".to_string()), None]
        );
        assert_eq!(
            state.load_source_suffixes,
            vec![Some("m4a".to_string()), None]
        );
    }

    #[test]
    fn raw_stream_output_device_error_does_not_fall_back_to_standard_stream() {
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
        backend_state.lock().unwrap().load_error = Some(
            "Audio output device error: failed to build output stream: unavailable".to_string(),
        );

        let result = controller.play(&runtime_context);
        let state = backend_state.lock().unwrap();

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Audio output device error"));
        assert!(!error.contains("fallback stream failed"));
        assert_eq!(state.load_calls.len(), 1);
        assert!(state.load_calls[0].contains("format=raw"));
    }

    #[test]
    fn transcoding_stream_uses_standard_stream_with_max_bit_rate() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Mp3,
                Some(&runtime_context),
            )
            .unwrap();
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(load_calls[0].contains("maxBitRate=192"));
        assert!(load_calls[0].contains("format=mp3"));
        assert!(load_calls[0].contains("estimateContentLength=true"));
        assert!(!load_calls[0].contains("format=raw"));
    }

    #[test]
    fn supported_custom_transcoding_codec_uses_requested_stream_format() {
        let (mut controller, backend_state, _) = controller_with_mock_backend_playback_capabilities(
            mock_playback_capabilities_with_codecs(
                true,
                vec![
                    PlaybackTranscodingCodec::Mp3,
                    PlaybackTranscodingCodec::Opus,
                ],
            ),
        );
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Opus,
                Some(&runtime_context),
            )
            .unwrap();
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(load_calls[0].contains("maxBitRate=192"));
        assert!(load_calls[0].contains("format=opus"));
    }

    #[test]
    fn unsupported_custom_transcoding_codec_falls_back_to_mp3_stream_format() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Opus,
                Some(&runtime_context),
            )
            .unwrap();
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(load_calls[0].contains("maxBitRate=192"));
        assert!(load_calls[0].contains("format=mp3"));
        assert!(!load_calls[0].contains("format=opus"));
    }

    #[test]
    fn custom_auto_transcoding_codec_omits_stream_format() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Auto,
                Some(&runtime_context),
            )
            .unwrap();
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(load_calls[0].contains("maxBitRate=192"));
        assert!(!load_calls[0].contains("format="));
    }

    #[test]
    fn stream_request_omits_estimated_content_length_when_backend_disables_it() {
        let (mut controller, backend_state, _) =
            controller_with_mock_backend_estimate_content_length(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_stream_settings(
                PlaybackStreamMode::Transcoding,
                192,
                PlaybackTranscodingCodec::Mp3,
                Some(&runtime_context),
            )
            .unwrap();
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(load_calls[0].contains("maxBitRate=192"));
        assert!(load_calls[0].contains("format=mp3"));
        assert!(!load_calls[0].contains("estimateContentLength="));
    }

    #[test]
    fn raw_stream_ignores_transcoding_bit_rate() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_stream_settings(
                PlaybackStreamMode::Raw,
                192,
                PlaybackTranscodingCodec::Opus,
                Some(&runtime_context),
            )
            .unwrap();
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        controller.play(&runtime_context).unwrap();

        let load_calls = backend_state.lock().unwrap().load_calls.clone();
        assert_eq!(load_calls.len(), 1);
        assert!(load_calls[0].contains("format=raw"));
        assert!(!load_calls[0].contains("maxBitRate=192"));
        assert!(!load_calls[0].contains("format=opus"));
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
    fn handled_load_failure_marker_is_stripped_from_playback_error() {
        let error = playback_error_from_load_failure(&format!(
            "Failed to load Android playback media: {HANDLED_PLAYBACK_ERROR_PREFIX}timeout"
        ));

        assert!(error.handled);
        assert_eq!(
            error.message,
            "Failed to load Android playback media: timeout"
        );
    }

    #[test]
    fn native_handled_error_keeps_error_state_but_marks_error_handled() {
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
            .push(PlaybackNativeEvent::Error {
                message: "network timeout".to_string(),
                handled: true,
            });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        let error = status.playback_error.unwrap();
        assert_eq!(status.playing_state, PlayingState::Error);
        assert_eq!(error.message, "network timeout");
        assert!(error.handled);
    }

    #[test]
    fn native_output_stream_error_reloads_current_track_at_current_position() {
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
        backend_state.lock().unwrap().current_position_ms = 6_543;
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Error {
                message: "Audio output device error: output stream lost: DeviceNotAvailable"
                    .to_string(),
                handled: false,
            });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_position_ms, 6_543);
        assert!(status.playback_error.is_none());
        assert_eq!(backend_state.stop_calls, stop_calls_before + 1);
        assert_eq!(backend_state.load_calls.len(), 2);
        assert_eq!(
            backend_state.load_absolute_start_positions_ms,
            vec![0, 6_543]
        );
    }

    #[test]
    fn repeated_native_output_stream_error_retries_until_recovery_limit() {
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

        for position_ms in [6_543, 7_777, 8_888] {
            backend_state.lock().unwrap().current_position_ms = position_ms;
            native_events
                .lock()
                .unwrap()
                .push(PlaybackNativeEvent::Error {
                    message: "Audio output device error: output stream lost: DeviceNotAvailable"
                        .to_string(),
                    handled: false,
                });
            controller
                .process_native_events(Some(&runtime_context))
                .unwrap();

            let status = controller.state();
            assert_eq!(status.playing_state, PlayingState::Playing);
            assert_eq!(status.current_position_ms, position_ms);
        }

        let load_calls_after_recovery_limit = backend_state.lock().unwrap().load_calls.len();
        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Error {
                message: "Audio output device error: output stream lost: DeviceNotAvailable"
                    .to_string(),
                handled: false,
            });
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();
        assert_eq!(status.playing_state, PlayingState::Error);
        assert!(status.playback_error.as_ref().is_some_and(|error| error
            .message
            .contains("automatic output stream recovery limit reached")));
        assert_eq!(
            backend_state.load_calls.len(),
            load_calls_after_recovery_limit
        );
    }

    #[test]
    fn native_output_stream_error_retry_succeeds_after_initial_reload_failure() {
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
        backend_state.lock().unwrap().current_position_ms = 8_765;
        backend_state.lock().unwrap().always_fail_load = true;

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Error {
                message: "Audio output device error: output stream lost: DeviceNotAvailable"
                    .to_string(),
                handled: false,
            });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Interrupted);
        assert_eq!(status.current_position_ms, 8_765);
        assert!(status.playback_error.is_none());

        backend_state.lock().unwrap().always_fail_load = false;
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_position_ms, 8_765);
        assert!(status.playback_error.is_none());
        assert_eq!(backend_state.lock().unwrap().load_calls.len(), 3);
    }

    #[test]
    fn native_output_stream_error_retry_limit_does_not_leave_playing_without_stream() {
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
        backend_state.lock().unwrap().current_position_ms = 8_765;
        backend_state.lock().unwrap().always_fail_load = true;

        native_events
            .lock()
            .unwrap()
            .push(PlaybackNativeEvent::Error {
                message: "Audio output device error: output stream lost: DeviceNotAvailable"
                    .to_string(),
                handled: false,
            });

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();
        let status = controller.state();
        assert_eq!(status.playing_state, PlayingState::Interrupted);
        assert_eq!(status.current_position_ms, 8_765);
        assert!(status.playback_error.is_none());

        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();
        controller
            .process_native_events(Some(&runtime_context))
            .unwrap();

        let status = controller.state();
        let error = status.playback_error.unwrap();
        assert_eq!(status.playing_state, PlayingState::Error);
        assert_eq!(status.current_position_ms, 8_765);
        assert!(error
            .message
            .contains("automatic output stream recovery failed after 3 attempts"));
        assert_eq!(backend_state.lock().unwrap().load_calls.len(), 4);
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
            .and_then(|status| status.playback_error.as_ref())
            .is_some_and(
                |error| !error.handled && error.message.contains("Failed to load playback stream.")
            ));
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
        assert_eq!(status.play_next_queue_len, 1);
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
        assert_eq!(status.play_next_queue_len, 2);
    }

    #[test]
    fn insert_after_current_appends_to_existing_play_next_queue() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(0))
            .unwrap();

        controller
            .insert_after_current(queue_entries(&["song-x"]))
            .unwrap();
        let status = controller
            .insert_after_current(queue_entries(&["song-y"]))
            .unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-a", "song-x", "song-y", "song-b", "song-c"]);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.play_next_queue_len, 2);
    }

    #[test]
    fn insert_after_current_preserves_playing_position_from_backend() {
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
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;
        let load_calls_before = backend_state.lock().unwrap().load_calls.len();
        backend_state.lock().unwrap().current_position_ms = 4_321;

        let status = controller
            .insert_after_current(queue_entries(&["song-x"]))
            .unwrap();
        let backend_state = backend_state.lock().unwrap();

        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_song_id, Some("song-a".to_string()));
        assert_eq!(status.current_position_ms, 4_321);
        assert_eq!(status.play_next_queue_len, 1);
        assert_eq!(backend_state.stop_calls, stop_calls_before);
        assert_eq!(backend_state.load_calls.len(), load_calls_before);
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
    fn next_consumes_first_play_next_queue_entry() {
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
        controller
            .insert_after_current(queue_entries(&["song-x", "song-y"]))
            .unwrap();

        let status = controller.next(&runtime_context).unwrap();

        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-x".to_string()));
        assert_eq!(status.play_next_queue_len, 1);
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
    fn append_to_queue_preserves_playing_position_from_backend() {
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
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;
        let load_calls_before = backend_state.lock().unwrap().load_calls.len();
        backend_state.lock().unwrap().current_position_ms = 5_678;

        let status = controller
            .append_to_queue(queue_entries(&["song-x"]))
            .unwrap();
        let backend_state = backend_state.lock().unwrap();

        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_position_ms, 5_678);
        assert_eq!(backend_state.stop_calls, stop_calls_before);
        assert_eq!(backend_state.load_calls.len(), load_calls_before);
    }

    #[test]
    fn active_connect_queue_update_preserves_current_loaded_track() {
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
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;
        let load_calls_before = backend_state.lock().unwrap().load_calls.len();
        backend_state.lock().unwrap().current_position_ms = 7_654;

        let status = controller
            .apply_connect_shared_state(
                Some(&runtime_context),
                ConnectPlaybackState {
                    playing_state: PlayingState::Playing,
                    queue: queue_entries(&["song-a", "song-b"]),
                    current_index: Some(0),
                    play_next_queue_len: 0,
                    current_position_ms: 0,
                    current_song_id: Some("song-a".to_string()),
                },
                true,
                true,
            )
            .unwrap();
        let backend_state = backend_state.lock().unwrap();

        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.queue.len(), 2);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_position_ms, 7_654);
        assert_eq!(backend_state.stop_calls, stop_calls_before);
        assert_eq!(backend_state.load_calls.len(), load_calls_before);
    }

    #[test]
    fn active_connect_queue_update_syncs_queue_state() {
        let (mut controller, backend_state, queue_sync_state) =
            controller_with_mock_backend_and_queue_sync();
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
        let synced_before = queue_sync_state.lock().unwrap().synced_states.len();
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        let status = controller
            .apply_connect_shared_state(
                Some(&runtime_context),
                ConnectPlaybackState {
                    playing_state: PlayingState::Playing,
                    queue: queue_entries(&["song-a", "song-b"]),
                    current_index: Some(0),
                    play_next_queue_len: 1,
                    current_position_ms: 0,
                    current_song_id: Some("song-a".to_string()),
                },
                true,
                true,
            )
            .unwrap();

        assert_eq!(status.queue.len(), 2);
        assert_eq!(status.play_next_queue_len, 1);
        assert_eq!(backend_state.lock().unwrap().stop_calls, stop_calls_before);
        let synced_states = queue_sync_state.lock().unwrap();
        assert_eq!(synced_states.synced_states.len(), synced_before + 1);
        let synced = synced_states.synced_states.last().unwrap();
        assert_eq!(synced.queue.len(), 2);
        assert_eq!(synced.play_next_queue_len, 1);
    }

    #[test]
    fn active_connect_transfer_update_loads_even_when_local_status_was_playing() {
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
        controller
            .apply_connect_shared_state(
                Some(&runtime_context),
                ConnectPlaybackState {
                    playing_state: PlayingState::Playing,
                    queue: queue_entries(&["song-a"]),
                    current_index: Some(0),
                    play_next_queue_len: 0,
                    current_position_ms: 4_321,
                    current_song_id: Some("song-a".to_string()),
                },
                false,
                false,
            )
            .unwrap();
        let load_calls_before = backend_state.lock().unwrap().load_calls.len();

        let status = controller
            .apply_connect_shared_state(
                Some(&runtime_context),
                ConnectPlaybackState {
                    playing_state: PlayingState::Playing,
                    queue: queue_entries(&["song-a"]),
                    current_index: Some(0),
                    play_next_queue_len: 0,
                    current_position_ms: 5_678,
                    current_song_id: Some("song-a".to_string()),
                },
                true,
                false,
            )
            .unwrap();

        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_position_ms, 5_678);
        assert_eq!(
            backend_state.lock().unwrap().load_calls.len(),
            load_calls_before + 1
        );
    }

    #[test]
    fn active_connect_clear_queue_stops_loaded_track() {
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
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        let status = controller
            .apply_connect_shared_state(
                Some(&runtime_context),
                ConnectPlaybackState {
                    playing_state: PlayingState::Idle,
                    queue: vec![],
                    current_index: None,
                    play_next_queue_len: 0,
                    current_position_ms: 0,
                    current_song_id: None,
                },
                true,
                true,
            )
            .unwrap();

        assert!(status.queue.is_empty());
        assert_eq!(status.playing_state, PlayingState::Idle);
        assert_eq!(status.current_index, None);
        assert_eq!(status.current_song_id, None);
        assert_eq!(
            backend_state.lock().unwrap().stop_calls,
            stop_calls_before + 1
        );
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
    fn move_queue_index_moves_item_and_keeps_current_entry() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(
                queue_entries(&["song-a", "song-b", "song-c", "song-d"]),
                Some(2),
            )
            .unwrap();
        controller.set_position(12_345).unwrap();

        let status = controller.move_queue_index(0, 2).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-b", "song-c", "song-a", "song-d"]);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-c".to_string()));
        assert_eq!(status.current_position_ms, 12_345);
    }

    #[test]
    fn move_queue_index_before_current_shifts_current_entry_right() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(
                queue_entries(&["song-a", "song-b", "song-c", "song-d"]),
                Some(1),
            )
            .unwrap();

        let status = controller.move_queue_index(3, 0).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-d", "song-a", "song-b", "song-c"]);
        assert_eq!(status.current_index, Some(2));
        assert_eq!(status.current_song_id, Some("song-b".to_string()));
    }

    #[test]
    fn move_current_queue_index_updates_current_index_without_reloading() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(1))
            .unwrap();
        controller.set_position(12_345).unwrap();
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        let status = controller.move_queue_index(1, 2).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-a", "song-c", "song-b"]);
        assert_eq!(status.current_index, Some(2));
        assert_eq!(status.current_song_id, Some("song-b".to_string()));
        assert_eq!(status.current_position_ms, 12_345);
        assert_eq!(backend_state.lock().unwrap().stop_calls, stop_calls_before);
    }

    #[test]
    fn move_queue_index_rejects_out_of_range_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        let result = controller.move_queue_index(0, 1);

        assert!(result.is_err());
    }

    #[test]
    fn remove_queue_index_after_current_keeps_current_entry() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(0))
            .unwrap();
        controller.set_position(12_345).unwrap();

        let status = controller.remove_queue_index(None, 2).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-a", "song-b"]);
        assert_eq!(status.current_index, Some(0));
        assert_eq!(status.current_song_id, Some("song-a".to_string()));
        assert_eq!(status.current_position_ms, 12_345);
        assert_eq!(status.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn remove_queue_index_before_current_shifts_current_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(2))
            .unwrap();
        controller.set_position(12_345).unwrap();

        let status = controller.remove_queue_index(None, 0).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-b", "song-c"]);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-c".to_string()));
        assert_eq!(status.current_position_ms, 12_345);
    }

    #[test]
    fn remove_current_queue_index_selects_next_entry_when_stopped() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(1))
            .unwrap();
        controller.set_position(12_345).unwrap();

        let status = controller.remove_queue_index(None, 1).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-a", "song-c"]);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-c".to_string()));
        assert_eq!(status.current_position_ms, 0);
        assert_eq!(status.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn remove_current_queue_index_selects_previous_entry_at_end() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(2))
            .unwrap();

        let status = controller.remove_queue_index(None, 2).unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(ids, vec!["song-a", "song-b"]);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-b".to_string()));
        assert_eq!(status.current_position_ms, 0);
    }

    #[test]
    fn remove_only_current_queue_index_resets_to_idle() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        let status = controller.remove_queue_index(None, 0).unwrap();

        assert!(status.queue.is_empty());
        assert_eq!(status.current_index, None);
        assert_eq!(status.current_song_id, None);
        assert_eq!(status.current_position_ms, 0);
        assert_eq!(status.playing_state, PlayingState::Idle);
    }

    #[test]
    fn remove_current_queue_index_loads_replacement_when_playing() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: None,
        };
        controller
            .set_queue(queue_entries(&["song-a", "song-b", "song-c"]), Some(1))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;
        backend_state.lock().unwrap().current_position_ms = 4_321;

        let status = controller
            .remove_queue_index(Some(&runtime_context), 1)
            .unwrap();
        let ids: Vec<_> = status.queue.iter().map(|entry| entry.id.as_str()).collect();
        let backend_state = backend_state.lock().unwrap();

        assert_eq!(ids, vec!["song-a", "song-c"]);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.current_song_id, Some("song-c".to_string()));
        assert_eq!(status.current_position_ms, 0);
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(backend_state.stop_calls, stop_calls_before + 1);
        assert_eq!(backend_state.load_calls.len(), 2);
        assert!(backend_state.load_calls[1].contains("song-c"));
    }

    #[test]
    fn remove_queue_index_rejects_out_of_range_index() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();

        let result = controller.remove_queue_index(None, 1);

        assert!(result.is_err());
    }

    #[test]
    fn clear_queue_stops_backend_and_resets_to_idle() {
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
        backend_state.lock().unwrap().current_position_ms = 12_345;
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        let status = controller.clear_queue(Some(&runtime_context)).unwrap();

        assert!(status.queue.is_empty());
        assert_eq!(status.current_index, None);
        assert_eq!(status.current_song_id, None);
        assert_eq!(status.current_position_ms, 0);
        assert_eq!(status.playing_state, PlayingState::Idle);
        assert_eq!(
            backend_state.lock().unwrap().stop_calls,
            stop_calls_before + 1
        );
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
    fn set_volume_clamps_and_forwards_to_backend() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);

        controller.set_volume(0.42).unwrap();
        controller.set_volume(5.0).unwrap();

        assert_eq!(
            backend_state.lock().unwrap().volume_calls,
            vec![1.0, 0.42, 1.0]
        );
    }

    #[test]
    fn set_output_device_while_stopped_updates_backend_without_reload() {
        let (mut controller, backend_state, _) = controller_with_mock_backend(false);
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        controller
            .set_output_device(Some("output:speakers".to_string()), None)
            .unwrap();

        let backend_state = backend_state.lock().unwrap();
        assert_eq!(
            backend_state.output_device_calls,
            vec![None, Some("output:speakers".to_string())]
        );
        assert_eq!(backend_state.stop_calls, stop_calls_before);
        assert!(backend_state.load_calls.is_empty());
    }

    #[test]
    fn initial_output_device_error_keeps_saved_effective_device_until_clear_succeeds() {
        let (mut controller, backend_state, _) = controller_with_initial_output_device(
            Some("output:temporarily-missing".to_string()),
            true,
        );

        controller
            .set_output_device(Some("output:temporarily-missing".to_string()), None)
            .unwrap();
        let default_switch_result = controller.set_output_device(None, None);

        assert!(default_switch_result.is_err());
        assert!(backend_state.lock().unwrap().output_device_calls.is_empty());
    }

    #[test]
    fn set_output_device_while_playing_reloads_current_track_at_current_position() {
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
        backend_state.lock().unwrap().current_position_ms = 4_321;
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        controller
            .set_output_device(
                Some("output:headphones".to_string()),
                Some(&runtime_context),
            )
            .unwrap();
        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();

        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(status.current_position_ms, 4_321);
        assert_eq!(backend_state.stop_calls, stop_calls_before + 1);
        assert_eq!(
            backend_state.output_device_calls.last(),
            Some(&Some("output:headphones".to_string()))
        );
        assert_eq!(backend_state.load_calls.len(), 2);
        assert_eq!(
            backend_state.load_absolute_start_positions_ms,
            vec![0, 4_321]
        );
    }

    #[test]
    fn set_output_device_while_paused_reloads_current_track_paused() {
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
        backend_state.lock().unwrap().current_position_ms = 2_222;

        controller
            .set_output_device(
                Some("output:headphones".to_string()),
                Some(&runtime_context),
            )
            .unwrap();
        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();

        assert_eq!(status.playing_state, PlayingState::Paused);
        assert_eq!(status.current_position_ms, 2_222);
        assert_eq!(backend_state.load_calls.len(), 2);
        assert_eq!(
            backend_state.load_absolute_start_positions_ms,
            vec![0, 2_222]
        );
    }

    #[test]
    fn set_output_device_clears_prepared_gapless_state() {
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
        let clear_calls_before = backend_state.lock().unwrap().clear_prepared_calls;

        controller
            .set_output_device(
                Some("output:headphones".to_string()),
                Some(&runtime_context),
            )
            .unwrap();

        assert!(backend_state.lock().unwrap().clear_prepared_calls > clear_calls_before);
    }

    #[test]
    fn set_output_device_backend_error_does_not_stop_active_playback() {
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
        backend_state.lock().unwrap().fail_output_device = true;
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        let result = controller.set_output_device(
            Some("output:headphones".to_string()),
            Some(&runtime_context),
        );
        let status = controller.state();

        assert!(result.is_err());
        assert_eq!(status.playing_state, PlayingState::Playing);
        assert_eq!(backend_state.lock().unwrap().stop_calls, stop_calls_before);
    }

    #[test]
    fn set_output_device_reload_error_rolls_back_and_does_not_leave_playing_without_stream() {
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
        backend_state.lock().unwrap().current_position_ms = 4_321;
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;
        backend_state.lock().unwrap().always_fail_load = true;

        let result = controller.set_output_device(
            Some("output:headphones".to_string()),
            Some(&runtime_context),
        );
        let status = controller.state();
        let backend_state = backend_state.lock().unwrap();

        assert!(result.is_err());
        assert_eq!(status.playing_state, PlayingState::Error);
        assert_eq!(status.current_position_ms, 4_321);
        assert!(status.playback_error.is_some());
        assert_eq!(backend_state.stop_calls, stop_calls_before + 1);
        assert_eq!(backend_state.output_device_calls.last(), Some(&None));
    }

    #[test]
    fn restore_state_sets_queue_and_stopped_state() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let queue = queue_entries(&["song-a", "song-b", "song-c"]);
        let status = controller.restore_state("profile-1".to_string(), queue, Some(1), 1, 5000);
        assert_eq!(status.queue.len(), 3);
        assert_eq!(status.current_index, Some(1));
        assert_eq!(status.play_next_queue_len, 1);
        assert_eq!(status.current_song_id, Some("song-b".to_string()));
        assert_eq!(status.current_position_ms, 5000);
        assert_eq!(status.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn restore_state_with_empty_queue_sets_idle() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let status = controller.restore_state("profile-1".to_string(), vec![], None, 0, 0);
        assert_eq!(status.playing_state, PlayingState::Idle);
        assert_eq!(status.queue.len(), 0);
    }

    #[test]
    fn restore_state_with_invalid_index_ignores_queue() {
        let (mut controller, _, _) = controller_with_mock_backend(false);
        let queue = queue_entries(&["song-a"]);
        let status = controller.restore_state("profile-1".to_string(), queue, Some(5), 0, 0);
        // Should start fresh - queue not restored
        assert_eq!(status.queue.len(), 0);
        assert_eq!(status.playing_state, PlayingState::Idle);
    }

    #[test]
    fn is_initialized_for_profile_tracks_active_profile() {
        let (mut controller, _, _) = controller_with_mock_backend(false);

        assert!(!controller.is_initialized_for_profile("profile-1"));

        controller.set_active_profile_id(Some("profile-1".to_string()));
        assert!(controller.is_initialized_for_profile("profile-1"));
        assert!(!controller.is_initialized_for_profile("profile-2"));
    }

    #[test]
    fn deferred_state_restore_is_not_initialized_or_persisted() {
        let persister = RecordingPersister::default();
        let saved_states = persister.states.clone();
        let (mut controller, _, _) =
            controller_with_mock_backend_and_persister(Box::new(persister));

        controller.defer_state_restore("profile-1".to_string());
        controller.sync_and_persist();

        assert!(!controller.is_initialized_for_profile("profile-1"));
        assert!(!controller.can_preserve_active_connect_snapshot_for_profile("profile-1"));
        assert!(saved_states.lock().unwrap().is_empty());
    }

    #[test]
    fn sync_and_persist_stops_loaded_backend_for_app_exit() {
        let persister = RecordingPersister::default();
        let saved_states = persister.states.clone();
        let (mut controller, backend_state, _) =
            controller_with_mock_backend_and_persister(Box::new(persister));
        let (client, capability_matrix) = runtime_context(true);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: Some("profile-1"),
        };
        controller.set_active_profile_id(Some("profile-1".to_string()));
        controller
            .set_queue(queue_entries(&["song-a"]), Some(0))
            .unwrap();
        controller.play(&runtime_context).unwrap();
        backend_state.lock().unwrap().current_position_ms = 12_345;
        let stop_calls_before = backend_state.lock().unwrap().stop_calls;

        controller.sync_and_persist();

        assert_eq!(
            backend_state.lock().unwrap().stop_calls,
            stop_calls_before + 1
        );
        let saved_states = saved_states.lock().unwrap();
        let saved_state = saved_states.last().unwrap();
        assert_eq!(saved_state.current_position_ms, 12_345);
    }

    #[test]
    fn connect_shared_state_clears_deferred_restore_and_persists() {
        let persister = RecordingPersister::default();
        let saved_states = persister.states.clone();
        let (mut controller, _, _) =
            controller_with_mock_backend_and_persister(Box::new(persister));
        let queue = queue_entries(&["song-a"]);
        let (client, capability_matrix) = runtime_context(false);
        let runtime_context = PlaybackRuntimeContext {
            client: &client,
            capability_matrix: &capability_matrix,
            cover_art_cache: None,
            profile_id: Some("profile-1"),
        };

        controller.defer_state_restore("profile-1".to_string());
        controller
            .apply_connect_shared_state(
                Some(&runtime_context),
                ConnectPlaybackState {
                    playing_state: PlayingState::Stopped,
                    queue,
                    current_index: Some(0),
                    play_next_queue_len: 0,
                    current_position_ms: 42,
                    current_song_id: Some("song-a".to_string()),
                },
                true,
                false,
            )
            .unwrap();

        assert!(controller.is_initialized_for_profile("profile-1"));
        assert!(controller.can_preserve_active_connect_snapshot_for_profile("profile-1"));
        let saved_states = saved_states.lock().unwrap();
        assert_eq!(saved_states.len(), 1);
        assert_eq!(saved_states[0].profile_id, "profile-1");
        assert_eq!(saved_states[0].queue[0].id, "song-a");
        assert_eq!(saved_states[0].current_index, Some(0));
    }
}
