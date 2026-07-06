//! Windows System Media Transport Controls (SMTC) integration.
//!
//! This makes Transonic show up as a media session in the OS: the volume-flyout
//! "now playing" card, the media overlay, lock screen, and keyboard media keys.
//! It is built on the cross-platform [`souvlaki`] crate; only obtaining the
//! `HWND` is Windows-specific.
//!
//! ## Layering
//!
//! SMTC does not touch the audio backend shim (`windows_symphonia.rs`). It sits
//! at the controller/reporting layer:
//! - **State → OS** rides the existing [`PlaybackReporter`](super::reporting)
//!   fan-out: [`SmtcReporter`] is one more reporter, wired in at the composition
//!   root (`create_playback_controller`) behind `#[cfg(windows)]`.
//! - **OS → app** (button presses) routes back through the ordinary
//!   `#[tauri::command]` playback entry points, so a media-key press behaves
//!   exactly like a UI button press (Connect routing, runtime context, and the
//!   fire-and-forget dispatch all come for free).
//!
//! ## Cover art
//!
//! The cover art cache already holds the current song's artwork on disk: the
//! player bar shows it via `getCoverArtCached`/`getCoverArt`, which return a
//! real filesystem path (`localPath`). SMTC reuses that exact cache — same key
//! (`cover_art_id`) and size (512, matching [`PlayerBar`]) — and hands the file
//! to souvlaki. Note souvlaki's Windows backend does NOT parse an RFC file URL:
//! it strips a literal `"file://"` prefix and passes the remainder straight to
//! `StorageFile::GetFileFromPathAsync`, so it needs `file://` + a raw Windows
//! path (backslashes, no percent-encoding), not `url::Url::from_file_path`'s
//! `file:///C:/...` form.
//!
//! ## Threading
//!
//! `MediaControls` wraps COM/WinRT objects bound to the window, so it must only
//! be created and touched from the main/UI thread that owns the window and its
//! message pump. It therefore lives in a thread-local ([`CONTROLS`]) on the main
//! thread. `report_state` is called from arbitrary background threads, so every
//! update is marshalled onto the main thread via
//! [`AppHandle::run_on_main_thread`]. `SmtcReporter` itself holds no COM state
//! (only an `AppHandle` and shared atomics), so it stays `Send`.

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use tauri::{AppHandle, Manager};

use super::reporting::PlaybackReporter;
use crate::models::{PlaybackSeekRequest, PlaybackStatus, PlayingState, SongResponse};
use crate::{ActiveSessionState, CoverArtCacheState};

/// Cover art size (px) to request for the SMTC thumbnail. Matches the player
/// bar (`CoverArtSizes.lg`) so the lookup hits the entry it already cached.
const SMTC_COVER_ART_SIZE: u32 = 512;

thread_local! {
    /// The live `MediaControls`, pinned to the main/UI thread. `MediaControls`
    /// holds COM interface pointers and must only be touched from this thread;
    /// all updates are marshalled here via `run_on_main_thread`. Kept alive for
    /// the process lifetime — dropping it detaches the controls.
    static CONTROLS: RefCell<Option<MediaControls>> = RefCell::new(None);
}

/// State shared between the (background-thread) reporter, the (background-thread)
/// cover-art task, and the (main-thread) OS event callback.
struct SmtcShared {
    /// Last playing state pushed to SMTC, used to resolve a `Toggle` press.
    is_playing: AtomicBool,
    /// Song id currently shown in SMTC; guards late-arriving cover art so that
    /// art for an already-superseded song is dropped.
    current_song_id: Mutex<Option<String>>,
}

/// Owned copy of the metadata needed to (re-)push a track to SMTC from another
/// thread.
#[derive(Clone)]
struct RenderMeta {
    song_id: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_secs: Option<u32>,
}

/// A [`PlaybackReporter`] that mirrors playback state into the Windows SMTC.
pub struct SmtcReporter {
    app: AppHandle,
    shared: Arc<SmtcShared>,
    /// Song id last rendered to SMTC, for de-duping position-only ticks.
    last_song_id: Option<String>,
    /// Playing state last rendered to SMTC, for de-duping position-only ticks.
    last_playing_state: Option<PlayingState>,
}

/// Create the SMTC session and return a reporter to wire into the composite.
///
/// Best-effort: any failure is logged and `None` is returned so the app runs
/// normally without OS media integration. Must be called on the main thread
/// (it is, from `create_playback_controller` during Tauri `setup`).
pub fn init(app: &AppHandle) -> Option<SmtcReporter> {
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => {
            log::error!("smtc: main window not found; SMTC disabled");
            return None;
        }
    };
    let hwnd = match window.hwnd() {
        Ok(handle) => handle.0 as *mut c_void,
        Err(error) => {
            log::error!("smtc: failed to obtain HWND: {error}; SMTC disabled");
            return None;
        }
    };

    let config = PlatformConfig {
        dbus_name: "transonic",
        display_name: "Transonic",
        hwnd: Some(hwnd),
    };
    let mut controls = match MediaControls::new(config) {
        Ok(controls) => controls,
        Err(error) => {
            log::error!("smtc: MediaControls::new failed: {error:?}; SMTC disabled");
            return None;
        }
    };

    let shared = Arc::new(SmtcShared {
        is_playing: AtomicBool::new(false),
        current_song_id: Mutex::new(None),
    });

    let callback_app = app.clone();
    let callback_shared = Arc::clone(&shared);
    if let Err(error) = controls.attach(move |event: MediaControlEvent| {
        handle_os_event(&callback_app, &callback_shared, event);
    }) {
        log::error!("smtc: attach failed: {error:?}; SMTC disabled");
        return None;
    }

    CONTROLS.with(|slot| *slot.borrow_mut() = Some(controls));
    log::info!("smtc: initialized");

    Some(SmtcReporter {
        app: app.clone(),
        shared,
        last_song_id: None,
        last_playing_state: None,
    })
}

impl PlaybackReporter for SmtcReporter {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
        let song = current_song(status);
        let playing_state = status.playing_state.clone();
        self.shared.is_playing.store(
            matches!(playing_state, PlayingState::Playing),
            Ordering::Relaxed,
        );

        let song_id = song.map(|song| song.id.clone());
        let song_changed = self.last_song_id != song_id;
        let state_changed = self.last_playing_state.as_ref() != Some(&playing_state);
        if !song_changed && !state_changed {
            // Position-only tick: don't re-push metadata every second.
            return Ok(());
        }
        self.last_song_id = song_id;
        self.last_playing_state = Some(playing_state.clone());
        // Record the currently-shown song so a late cover task for a superseded
        // song is dropped. Set on every song change — including transitions to a
        // song with no cover art or to `None` (stop/empty queue), where no cover
        // task is spawned — so `current_song_id` always reflects what SMTC shows.
        if song_changed {
            *self.shared.current_song_id.lock().unwrap() = self.last_song_id.clone();
        }

        let meta = song.map(|song| RenderMeta {
            song_id: song.id.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            duration_secs: song.duration,
        });
        let playback = map_playback(&playing_state, status.current_position_ms);

        let render_meta = meta.clone();
        if let Err(error) = self
            .app
            .run_on_main_thread(move || render(render_meta.as_ref(), playback, song_changed))
        {
            log::warn!("smtc: run_on_main_thread failed: {error}");
        }

        if song_changed {
            if let (Some(meta), Some(song)) = (meta, song) {
                // Match the player bar's key: the plain `cover_art_id`.
                if let Some(cover_art_id) = song.cover_art_id.clone() {
                    spawn_cover_art_update(&self.app, Arc::clone(&self.shared), cover_art_id, meta);
                }
            }
        }

        Ok(())
    }
}

/// Map a button/media-key press back onto the ordinary playback commands.
fn handle_os_event(app: &AppHandle, shared: &SmtcShared, event: MediaControlEvent) {
    use crate::commands::{
        playback_next, playback_pause, playback_play, playback_prev, playback_seek, playback_stop,
    };

    match event {
        MediaControlEvent::Play => {
            let _ = playback_play(app.clone());
        }
        MediaControlEvent::Pause => {
            let _ = playback_pause(app.clone());
        }
        MediaControlEvent::Stop => {
            let _ = playback_stop(app.clone());
        }
        MediaControlEvent::Toggle => {
            if shared.is_playing.load(Ordering::Relaxed) {
                let _ = playback_pause(app.clone());
            } else {
                let _ = playback_play(app.clone());
            }
        }
        MediaControlEvent::Next => {
            let _ = playback_next(app.clone());
        }
        MediaControlEvent::Previous => {
            let _ = playback_prev(app.clone());
        }
        MediaControlEvent::SetPosition(MediaPosition(position)) => {
            let position_ms = position.as_millis().min(u128::from(u32::MAX)) as u32;
            let _ = playback_seek(app.clone(), PlaybackSeekRequest { position_ms });
        }
        MediaControlEvent::Raise => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }
        other => {
            // Seek/SeekBy/SetVolume/OpenUri/Quit are not mapped (v1).
            log::debug!("smtc: unhandled event {other:?}");
        }
    }
}

/// Push the current track and playback state to SMTC. Runs on the main thread.
///
/// `update_metadata` must be `true` only when the song changed. On a plain
/// play/pause tick it is `false` so we update the transport state without
/// re-sending metadata (which would blow away the async-loaded cover art).
fn render(meta: Option<&RenderMeta>, playback: MediaPlayback, update_metadata: bool) {
    CONTROLS.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(controls) = guard.as_mut() else {
            return;
        };

        match meta {
            Some(meta) => {
                if update_metadata {
                    let _ = controls.set_metadata(MediaMetadata {
                        title: Some(meta.title.as_str()),
                        album: meta.album.as_deref(),
                        artist: meta.artist.as_deref(),
                        cover_url: None,
                        duration: meta
                            .duration_secs
                            .map(|secs| Duration::from_secs(u64::from(secs))),
                    });
                }
                let _ = controls.set_playback(playback);
            }
            None => {
                let _ = controls.set_playback(MediaPlayback::Stopped);
            }
        }
    });
}

/// Resolve the current song's cover art path (reusing the on-disk cache) and,
/// if that song is still the one shown, re-push metadata with the artwork.
fn spawn_cover_art_update(
    app: &AppHandle,
    shared: Arc<SmtcShared>,
    cover_art_id: String,
    meta: RenderMeta,
) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = resolve_cover_art_path(&app, &cover_art_id) else {
            return;
        };
        // souvlaki's Windows backend strips a literal "file://" prefix and passes
        // the remainder straight to StorageFile::GetFileFromPathAsync, so it needs
        // a raw Windows path (backslashes, no percent-encoding) — NOT an RFC file
        // URL. `url::Url::from_file_path` would produce `file:///C:/...`, whose
        // stripped remainder `/C:/...` is not a valid path and silently fails.
        let cover_url = format!("file://{}", path.to_string_lossy());

        if let Err(error) = app.run_on_main_thread(move || {
            if shared.current_song_id.lock().unwrap().as_deref() != Some(meta.song_id.as_str()) {
                return; // song changed while the art was loading; drop it
            }
            CONTROLS.with(|slot| {
                if let Some(controls) = slot.borrow_mut().as_mut() {
                    let _ = controls.set_metadata(MediaMetadata {
                        title: Some(meta.title.as_str()),
                        album: meta.album.as_deref(),
                        artist: meta.artist.as_deref(),
                        cover_url: Some(cover_url.as_str()),
                        duration: meta
                            .duration_secs
                            .map(|secs| Duration::from_secs(u64::from(secs))),
                    });
                }
            });
        }) {
            log::warn!("smtc: run_on_main_thread (cover art) failed: {error}");
        }
    });
}

/// Resolve a cover art file path, preferring the already-cached entry (same key
/// and size the player bar uses) and only downloading if it isn't cached yet.
/// Blocking.
fn resolve_cover_art_path(app: &AppHandle, cover_art_id: &str) -> Option<PathBuf> {
    let sessions = app.try_state::<ActiveSessionState>()?;
    let profile_id = {
        let guard = sessions.0.lock().ok()?;
        guard.as_ref()?.profile_id.clone()
    };
    let cache = app.try_state::<CoverArtCacheState>()?.0.clone();

    if let Ok(Some(path)) =
        cache.resolve_cached_cover_art(&profile_id, cover_art_id, Some(SMTC_COVER_ART_SIZE), &[])
    {
        return Some(path);
    }

    let client = crate::commands::common::client(app, &sessions.0).ok()?;
    cache
        .resolve_cover_art(&client, &profile_id, cover_art_id, Some(SMTC_COVER_ART_SIZE))
        .ok()
}

fn current_song(status: &PlaybackStatus) -> Option<&SongResponse> {
    let index = usize::try_from(status.current_index?).ok()?;
    status.queue.get(index)
}

fn map_playback(state: &PlayingState, position_ms: u32) -> MediaPlayback {
    let progress = Some(MediaPosition(Duration::from_millis(u64::from(position_ms))));
    match state {
        PlayingState::Playing => MediaPlayback::Playing { progress },
        PlayingState::Paused | PlayingState::Interrupted => MediaPlayback::Paused { progress },
        PlayingState::Idle | PlayingState::Stopped | PlayingState::Error => MediaPlayback::Stopped,
    }
}
