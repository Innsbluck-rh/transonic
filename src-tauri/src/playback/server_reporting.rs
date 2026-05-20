use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    time::{Duration, Instant},
};

use opensubsonic_client::{
    api::annotation::{AnnotationApi, ReportPlaybackRequest, ScrobbleRequest},
    MediaType, OpenSubsonicClient, PlaybackState as OpenSubsonicPlaybackState,
};

use crate::models::CapabilityMatrix;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const FALLBACK_SCROBBLE_MAX_THRESHOLD_MS: u64 = 4 * 60 * 1000;
const FALLBACK_SCROBBLE_UNKNOWN_DURATION_THRESHOLD_MS: u64 = 30 * 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerPlaybackState {
    Starting,
    Playing,
    Paused,
    Stopped,
}

impl ServerPlaybackState {
    fn as_api_state(&self) -> OpenSubsonicPlaybackState {
        match self {
            Self::Starting => OpenSubsonicPlaybackState::Starting,
            Self::Playing => OpenSubsonicPlaybackState::Playing,
            Self::Paused => OpenSubsonicPlaybackState::Paused,
            Self::Stopped => OpenSubsonicPlaybackState::Stopped,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerPlaybackReportingContext {
    pub client: OpenSubsonicClient,
    pub capability_matrix: CapabilityMatrix,
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPlaybackTrack {
    pub song_id: String,
    pub media_type: MediaType,
    pub queue_index: Option<u32>,
    pub duration_ms: Option<u64>,
}

pub trait PlaybackServerReporter: Send {
    fn report_state(
        &mut self,
        context: ServerPlaybackReportingContext,
        track: ServerPlaybackTrack,
        position_ms: u32,
        state: ServerPlaybackState,
    );

    fn clear(&mut self);
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Default)]
pub struct NoopPlaybackServerReporter;

impl PlaybackServerReporter for NoopPlaybackServerReporter {
    fn report_state(
        &mut self,
        _context: ServerPlaybackReportingContext,
        _track: ServerPlaybackTrack,
        _position_ms: u32,
        _state: ServerPlaybackState,
    ) {
    }

    fn clear(&mut self) {}
}

#[derive(Debug)]
pub struct BackgroundPlaybackServerReporter {
    tx: Sender<ServerPlaybackCommand>,
}

#[derive(Debug)]
enum ServerPlaybackCommand {
    Report {
        context: ServerPlaybackReportingContext,
        track: ServerPlaybackTrack,
        position_ms: u32,
        state: ServerPlaybackState,
    },
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FallbackTrackKey {
    profile_id: String,
    song_id: String,
    queue_index: Option<u32>,
}

#[derive(Debug, Clone)]
struct ActiveReportPlaybackSession {
    context: ServerPlaybackReportingContext,
    track: ServerPlaybackTrack,
    state: ServerPlaybackState,
    position_ms: u64,
    position_updated_at: Instant,
}

impl ActiveReportPlaybackSession {
    fn new(
        context: ServerPlaybackReportingContext,
        track: ServerPlaybackTrack,
        position_ms: u32,
        state: ServerPlaybackState,
    ) -> Self {
        Self {
            context,
            track,
            state,
            position_ms: u64::from(position_ms),
            position_updated_at: Instant::now(),
        }
    }

    fn next_timeout(&self) -> Duration {
        if self.state != ServerPlaybackState::Playing {
            return IDLE_TIMEOUT;
        }

        let elapsed = self.position_updated_at.elapsed();
        if elapsed >= HEARTBEAT_INTERVAL {
            Duration::ZERO
        } else {
            HEARTBEAT_INTERVAL - elapsed
        }
    }

    fn heartbeat_position_ms(&self) -> u64 {
        if self.state != ServerPlaybackState::Playing {
            return self.position_ms;
        }

        self.position_ms
            .saturating_add(self.position_updated_at.elapsed().as_millis() as u64)
    }
}

#[derive(Debug, Clone)]
struct ActiveFallbackScrobbleSession {
    context: ServerPlaybackReportingContext,
    track: ServerPlaybackTrack,
    state: ServerPlaybackState,
    position_ms: u64,
    position_updated_at: Instant,
    played_ms: u64,
    now_playing_reported: bool,
    played_scrobbled: bool,
}

impl ActiveFallbackScrobbleSession {
    fn new(
        context: ServerPlaybackReportingContext,
        track: ServerPlaybackTrack,
        position_ms: u32,
        state: ServerPlaybackState,
    ) -> Self {
        Self {
            context,
            track,
            state,
            position_ms: u64::from(position_ms),
            position_updated_at: Instant::now(),
            played_ms: 0,
            now_playing_reported: false,
            played_scrobbled: false,
        }
    }

    fn key(&self) -> FallbackTrackKey {
        fallback_track_key(&self.context, &self.track)
    }

    fn refresh_progress(&mut self) {
        if self.state == ServerPlaybackState::Playing {
            let elapsed_ms = self.position_updated_at.elapsed().as_millis() as u64;
            self.played_ms = self.played_ms.saturating_add(elapsed_ms);
            self.position_ms = self.position_ms.saturating_add(elapsed_ms);
        }
        self.position_updated_at = Instant::now();
    }

    fn update(&mut self, position_ms: u32, state: ServerPlaybackState) {
        self.refresh_progress();
        self.position_ms = u64::from(position_ms);
        self.state = state;
    }

    fn mark_now_playing_reported(&mut self) {
        self.now_playing_reported = true;
    }

    fn should_submit_played_scrobble(&mut self) -> bool {
        self.refresh_progress();
        should_submit_fallback_scrobble(
            self.played_ms,
            self.track.duration_ms,
            self.played_scrobbled,
        )
    }

    fn mark_played_scrobbled(&mut self) {
        self.played_scrobbled = true;
    }
}

fn fallback_track_key(
    context: &ServerPlaybackReportingContext,
    track: &ServerPlaybackTrack,
) -> FallbackTrackKey {
    FallbackTrackKey {
        profile_id: context.profile_id.clone(),
        song_id: track.song_id.clone(),
        queue_index: track.queue_index,
    }
}

fn fallback_scrobble_threshold_ms(duration_ms: Option<u64>) -> u64 {
    duration_ms
        .filter(|duration_ms| *duration_ms > 0)
        .map(|duration_ms| (duration_ms / 2).min(FALLBACK_SCROBBLE_MAX_THRESHOLD_MS))
        .unwrap_or(FALLBACK_SCROBBLE_UNKNOWN_DURATION_THRESHOLD_MS)
}

fn should_submit_fallback_scrobble(
    played_ms: u64,
    duration_ms: Option<u64>,
    already_scrobbled: bool,
) -> bool {
    !already_scrobbled && played_ms >= fallback_scrobble_threshold_ms(duration_ms)
}

impl BackgroundPlaybackServerReporter {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<ServerPlaybackCommand>();

        std::thread::Builder::new()
            .name("playback-server-reporter".into())
            .spawn(move || {
                let mut active_report_playback: Option<ActiveReportPlaybackSession> = None;
                let mut active_fallback_scrobble: Option<ActiveFallbackScrobbleSession> = None;

                loop {
                    let timeout = active_report_playback
                        .as_ref()
                        .map(ActiveReportPlaybackSession::next_timeout)
                        .unwrap_or(IDLE_TIMEOUT);

                    match rx.recv_timeout(timeout) {
                        Ok(ServerPlaybackCommand::Report {
                            context,
                            track,
                            position_ms,
                            state,
                        }) => {
                            if context.capability_matrix.playback_report {
                                spawn_report_playback(
                                    context.clone(),
                                    track.clone(),
                                    u64::from(position_ms),
                                    state.clone(),
                                );

                                active_report_playback = match state {
                                    ServerPlaybackState::Stopped => None,
                                    _ => Some(ActiveReportPlaybackSession::new(
                                        context,
                                        track,
                                        position_ms,
                                        state,
                                    )),
                                };
                                active_fallback_scrobble = None;
                                continue;
                            }

                            handle_fallback_report(
                                &mut active_fallback_scrobble,
                                context,
                                track,
                                position_ms,
                                state,
                            );

                            active_report_playback = None;
                        }
                        Ok(ServerPlaybackCommand::Clear) => {
                            active_report_playback = None;
                            active_fallback_scrobble = None;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            let Some(session) = active_report_playback.as_mut() else {
                                continue;
                            };
                            if session.state != ServerPlaybackState::Playing {
                                continue;
                            }

                            let position_ms = session.heartbeat_position_ms();
                            spawn_report_playback(
                                session.context.clone(),
                                session.track.clone(),
                                position_ms,
                                ServerPlaybackState::Playing,
                            );
                            session.position_ms = position_ms;
                            session.position_updated_at = Instant::now();
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .expect("Failed to spawn playback server reporter thread");

        Self { tx }
    }
}

impl Default for BackgroundPlaybackServerReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackServerReporter for BackgroundPlaybackServerReporter {
    fn report_state(
        &mut self,
        context: ServerPlaybackReportingContext,
        track: ServerPlaybackTrack,
        position_ms: u32,
        state: ServerPlaybackState,
    ) {
        let _ = self.tx.send(ServerPlaybackCommand::Report {
            context,
            track,
            position_ms,
            state,
        });
    }

    fn clear(&mut self) {
        let _ = self.tx.send(ServerPlaybackCommand::Clear);
    }
}

fn handle_fallback_report(
    active_session: &mut Option<ActiveFallbackScrobbleSession>,
    context: ServerPlaybackReportingContext,
    track: ServerPlaybackTrack,
    position_ms: u32,
    state: ServerPlaybackState,
) {
    let next_key = fallback_track_key(&context, &track);
    if active_session
        .as_ref()
        .is_some_and(|session| session.key() != next_key)
    {
        finalize_fallback_scrobble(active_session);
    }

    if active_session.is_none() {
        *active_session = Some(ActiveFallbackScrobbleSession::new(
            context,
            track,
            position_ms,
            state.clone(),
        ));
    } else if let Some(session) = active_session.as_mut() {
        session.update(position_ms, state.clone());
    }

    match state {
        ServerPlaybackState::Playing => {
            if let Some(session) = active_session.as_mut() {
                if !session.now_playing_reported {
                    spawn_now_playing_scrobble(session.context.clone(), session.track.clone());
                    session.mark_now_playing_reported();
                }
                submit_fallback_scrobble_if_ready(session);
            }
        }
        ServerPlaybackState::Paused => {
            if let Some(session) = active_session.as_mut() {
                submit_fallback_scrobble_if_ready(session);
            }
        }
        ServerPlaybackState::Stopped => {
            finalize_fallback_scrobble(active_session);
        }
        ServerPlaybackState::Starting => {}
    }
}

fn finalize_fallback_scrobble(active_session: &mut Option<ActiveFallbackScrobbleSession>) {
    let Some(session) = active_session.as_mut() else {
        return;
    };
    submit_fallback_scrobble_if_ready(session);
    *active_session = None;
}

fn submit_fallback_scrobble_if_ready(session: &mut ActiveFallbackScrobbleSession) {
    if session.should_submit_played_scrobble() {
        spawn_played_scrobble(session.context.clone(), session.track.clone());
        session.mark_played_scrobbled();
    }
}

fn spawn_report_playback(
    context: ServerPlaybackReportingContext,
    track: ServerPlaybackTrack,
    position_ms: u64,
    state: ServerPlaybackState,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = context
            .client
            .report_playback(ReportPlaybackRequest {
                media_id: track.song_id.clone(),
                media_type: track.media_type,
                position_ms,
                state: state.as_api_state(),
                playback_rate: Some(1.0),
                ignore_scrobble: Some(false),
            })
            .await
        {
            log::warn!(
                "server_reporting: reportPlayback failed for profile={} song_id={}: {:?}",
                context.profile_id,
                track.song_id,
                error
            );
        }
    });
}

fn spawn_now_playing_scrobble(context: ServerPlaybackReportingContext, track: ServerPlaybackTrack) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = context
            .client
            .scrobble(ScrobbleRequest {
                ids: vec![track.song_id.clone()],
                time: None,
                submission: Some(false),
            })
            .await
        {
            log::warn!(
                "server_reporting: scrobble(now playing) failed for profile={} song_id={}: {:?}",
                context.profile_id,
                track.song_id,
                error
            );
        }
    });
}

fn spawn_played_scrobble(context: ServerPlaybackReportingContext, track: ServerPlaybackTrack) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = context
            .client
            .scrobble(ScrobbleRequest {
                ids: vec![track.song_id.clone()],
                time: None,
                submission: Some(true),
            })
            .await
        {
            log::warn!(
                "server_reporting: scrobble(played) failed for profile={} song_id={}: {:?}",
                context.profile_id,
                track.song_id,
                error
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        fallback_scrobble_threshold_ms, should_submit_fallback_scrobble,
        FALLBACK_SCROBBLE_MAX_THRESHOLD_MS, FALLBACK_SCROBBLE_UNKNOWN_DURATION_THRESHOLD_MS,
    };

    #[test]
    fn fallback_scrobble_threshold_uses_half_duration_capped_at_four_minutes() {
        assert_eq!(fallback_scrobble_threshold_ms(Some(180_000)), 90_000);
        assert_eq!(
            fallback_scrobble_threshold_ms(Some(20 * 60 * 1000)),
            FALLBACK_SCROBBLE_MAX_THRESHOLD_MS
        );
    }

    #[test]
    fn fallback_scrobble_threshold_uses_unknown_duration_default() {
        assert_eq!(
            fallback_scrobble_threshold_ms(None),
            FALLBACK_SCROBBLE_UNKNOWN_DURATION_THRESHOLD_MS
        );
        assert_eq!(
            fallback_scrobble_threshold_ms(Some(0)),
            FALLBACK_SCROBBLE_UNKNOWN_DURATION_THRESHOLD_MS
        );
    }

    #[test]
    fn fallback_scrobble_submission_requires_threshold_and_no_prior_submission() {
        assert!(!should_submit_fallback_scrobble(
            89_999,
            Some(180_000),
            false
        ));
        assert!(should_submit_fallback_scrobble(
            90_000,
            Some(180_000),
            false
        ));
        assert!(!should_submit_fallback_scrobble(
            90_000,
            Some(180_000),
            true
        ));
    }
}
