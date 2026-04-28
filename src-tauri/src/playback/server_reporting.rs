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
struct ActiveFallbackTrack {
    profile_id: String,
    song_id: String,
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

impl BackgroundPlaybackServerReporter {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<ServerPlaybackCommand>();

        std::thread::Builder::new()
            .name("playback-server-reporter".into())
            .spawn(move || {
                let mut active_report_playback: Option<ActiveReportPlaybackSession> = None;
                let mut active_fallback_track: Option<ActiveFallbackTrack> = None;

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
                                active_fallback_track = None;
                                continue;
                            }

                            match state {
                                ServerPlaybackState::Playing => {
                                    let next_active_track = ActiveFallbackTrack {
                                        profile_id: context.profile_id.clone(),
                                        song_id: track.song_id.clone(),
                                    };

                                    if active_fallback_track.as_ref() != Some(&next_active_track) {
                                        spawn_now_playing_scrobble(context, track);
                                        active_fallback_track = Some(next_active_track);
                                    }
                                }
                                ServerPlaybackState::Stopped => {
                                    active_fallback_track = None;
                                }
                                ServerPlaybackState::Starting | ServerPlaybackState::Paused => {}
                            }

                            active_report_playback = None;
                        }
                        Ok(ServerPlaybackCommand::Clear) => {
                            active_report_playback = None;
                            active_fallback_track = None;
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
                ignore_scrobble: Some(true),
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

fn spawn_now_playing_scrobble(
    context: ServerPlaybackReportingContext,
    track: ServerPlaybackTrack,
) {
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
