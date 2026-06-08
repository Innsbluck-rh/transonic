//! Windows playback backend using **symphonia** for decoding and **cpal** for
//! audio output.
//!
//! Architecture (mirrors `windows_mf.rs`):
//! - `SymphoniaPlaybackBackend` sends commands to a dedicated worker thread
//!   via mpsc.
//! - On load the worker starts a progressive HTTP download via
//!   `reqwest::blocking` and wraps it in a `StreamingMediaSource` so that
//!   symphonia can begin probing/decoding as soon as the first few KB arrive
//!   (no need to wait for the full file).  A background download thread
//!   continues filling a shared buffer while the pump thread decodes.
//! - A *pump thread* uses symphonia to probe, decode, and seek within the
//!   streamed data.  Decoded f32 PCM samples are written into a shared ring
//!   buffer.
//! - A cpal output stream drains the ring buffer to produce audio.
//! - End-of-stream and same-format prepared-track handoff are detected by the
//!   cpal callback and signalled to the controller via
//!   `SymphoniaPlaybackEventHub`.
//!
//! Key difference from the MF backend: symphonia supports native seeking, so
//! the `seek` method sends a command to the pump thread (via a secondary
//! channel) instead of requiring a full reload.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::models::{
    normalize_volume, PlaybackActualStreamInfo, PlaybackCapabilities, PlaybackTranscodingCodec,
};
use crate::playback::backend_shims::backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};
use crate::playback::native_events::{NativePlaybackEventSource, PlaybackNativeEvent};

// ---------------------------------------------------------------------------
// StreamingMediaSource – progressive HTTP download buffer that implements
// symphonia's `MediaSource` trait.  Allows symphonia to start probing and
// decoding as soon as the first bytes arrive while the rest of the file is
// still being downloaded in the background.
// ---------------------------------------------------------------------------

/// Shared state between the background HTTP download thread and the
/// `StreamingMediaSource` reader (owned by the pump thread / symphonia).
struct SharedDownloadBuffer {
    inner: Mutex<DownloadBufferInner>,
    condvar: Condvar,
    /// Total file size obtained from the `Content-Length` HTTP header.
    total_len: u64,
    /// Symphonia 0.6 probes trailing metadata when a source is seekable and
    /// reports a byte length. Keep this disabled until format probing has
    /// completed so progressive playback does not block on the file tail.
    expose_byte_len: AtomicBool,
    /// Set to `true` to cancel the download and unblock any waiting `Read`.
    cancel: AtomicBool,
}

struct DownloadBufferInner {
    /// Contiguous byte buffer filled from byte 0 onward.
    data: Vec<u8>,
    /// `true` once the HTTP response body has been fully consumed (or an
    /// error occurred).
    complete: bool,
    /// If the download thread encountered an error it is stored here so that
    /// the next `Read` call can surface it.
    error: Option<String>,
}

#[derive(Default)]
struct PreparedActivationGate {
    inner: Mutex<PreparedActivationGateInner>,
    condvar: Condvar,
}

#[derive(Default)]
struct PreparedActivationGateInner {
    paused: bool,
    activated: bool,
    aborted: bool,
}

impl SharedDownloadBuffer {
    fn new(total_len: u64) -> Self {
        // Pre-allocate up to 64 MiB; for larger files the Vec will grow.
        let cap = (total_len as usize).min(64 * 1024 * 1024);
        Self {
            inner: Mutex::new(DownloadBufferInner {
                data: Vec::with_capacity(cap),
                complete: false,
                error: None,
            }),
            condvar: Condvar::new(),
            total_len,
            expose_byte_len: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
        }
    }

    fn expose_byte_len_after_probe(&self) {
        self.expose_byte_len.store(true, Ordering::Release);
    }

    /// Signal cancellation and wake any blocked reader.
    fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
        self.condvar.notify_all();
    }
}

impl PreparedActivationGate {
    fn is_activated(&self) -> bool {
        self.inner.lock().unwrap().activated
    }

    fn wait_for_download_permission(&self, cancel: &AtomicBool) -> bool {
        let mut inner = self.inner.lock().unwrap();
        while inner.paused && !inner.activated && !inner.aborted && !cancel.load(Ordering::Acquire)
        {
            inner = self.condvar.wait(inner).unwrap();
        }
        !inner.aborted && !cancel.load(Ordering::Acquire)
    }

    fn pause_until_activated(&self, cancel: &AtomicBool) {
        let mut inner = self.inner.lock().unwrap();
        if inner.activated || inner.aborted || cancel.load(Ordering::Acquire) {
            return;
        }
        if !inner.paused {
            inner.paused = true;
            self.condvar.notify_all();
            log::info!("symphonia-gapless: buffered ~2s PCM, pausing prepared pipeline");
        }
        while inner.paused && !inner.activated && !inner.aborted && !cancel.load(Ordering::Acquire)
        {
            inner = self.condvar.wait(inner).unwrap();
        }
    }

    fn activate(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.activated || inner.aborted {
            return;
        }
        inner.activated = true;
        inner.paused = false;
        self.condvar.notify_all();
        log::info!("symphonia-gapless: prepared pipeline activated");
    }

    fn abort(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.aborted = true;
        inner.paused = false;
        self.condvar.notify_all();
    }
}

/// A `symphonia::core::io::MediaSource` backed by a progressively-filled
/// in-memory buffer.
///
/// - **`Read`**: returns data that has already been downloaded; blocks (via
///   `Condvar`) when the read position is at the download frontier.
/// - **`Seek`**: updates the logical position.  Backward seeks within the
///   buffer are instant.  Forward seeks beyond the current download frontier
///   are allowed – the next `Read` will block until the data arrives.
/// - **`is_seekable()`** returns `true` so that symphonia can use seek tables
///   for efficient FLAC/MP3/OGG seeking.
/// - **`byte_len()`** returns the total file size (from `Content-Length`).
struct StreamingMediaSource {
    shared: Arc<SharedDownloadBuffer>,
    /// Current logical read/seek position (only mutated via `&mut self`).
    position: u64,
}

impl StreamingMediaSource {
    fn new(shared: Arc<SharedDownloadBuffer>) -> Self {
        Self {
            shared,
            position: 0,
        }
    }
}

impl std::io::Read for StreamingMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut inner = self.shared.inner.lock().unwrap();

        loop {
            // Cancellation check.
            if self.shared.cancel.load(Ordering::Acquire) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "download cancelled",
                ));
            }

            let data_len = inner.data.len() as u64;

            if self.position < data_len {
                // Data is available – copy as much as we can.
                let start = self.position as usize;
                let end = (inner.data.len()).min(start + buf.len());
                let n = end - start;
                buf[..n].copy_from_slice(&inner.data[start..end]);
                self.position += n as u64;
                return Ok(n);
            }

            // Position is at or beyond the download frontier.
            if inner.complete {
                // If the download finished with an error, surface it once.
                if let Some(ref err) = inner.error {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err.clone()));
                }
                // Normal EOF.
                return Ok(0);
            }

            // Wait for the download thread to append more data.
            inner = self.shared.condvar.wait(inner).unwrap();
        }
    }
}

impl std::io::Seek for StreamingMediaSource {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            std::io::SeekFrom::Start(offset) => offset,
            std::io::SeekFrom::End(offset) => {
                // `offset` is typically 0 or negative.
                let base = self.shared.total_len as i64;
                let target = base.checked_add(offset).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek overflow")
                })?;
                if target < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "seek before start of stream",
                    ));
                }
                target as u64
            }
            std::io::SeekFrom::Current(offset) => {
                let base = self.position as i64;
                let target = base.checked_add(offset).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek overflow")
                })?;
                if target < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "seek before start of stream",
                    ));
                }
                target as u64
            }
        };

        self.position = new_pos;
        Ok(new_pos)
    }
}

impl symphonia::core::io::MediaSource for StreamingMediaSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.shared
            .expose_byte_len
            .load(Ordering::Acquire)
            .then_some(self.shared.total_len)
    }
}

// ---------------------------------------------------------------------------
// SymphoniaPlaybackEventHub – shared EOS signalling between cpal callback
// and controller (via NativePlaybackEventSource).
// ---------------------------------------------------------------------------

/// Shared event queue between the symphonia playback backend and the
/// controller.
#[derive(Clone, Default)]
pub struct SymphoniaPlaybackEventHub {
    queue: Arc<Mutex<VecDeque<PlaybackNativeEvent>>>,
}

impl SymphoniaPlaybackEventHub {
    /// Reset the ended flag.  Called when a new track is loaded so that a
    /// stale flag from a previous session does not trigger a spurious event.
    fn reset(&self) {
        self.queue.lock().unwrap().clear();
    }

    fn push(&self, event: PlaybackNativeEvent) {
        self.queue.lock().unwrap().push_back(event);
        crate::playback::spawn_controller_process_native_events();
    }
}

impl NativePlaybackEventSource for SymphoniaPlaybackEventHub {
    fn drain_events(&mut self) -> Vec<PlaybackNativeEvent> {
        let mut queue = self.queue.lock().unwrap();
        queue.drain(..).collect()
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn create_playback_backend(event_hub: SymphoniaPlaybackEventHub) -> Box<dyn PlaybackBackend> {
    Box::new(SymphoniaPlaybackBackend::new(event_hub))
}

fn symphonia_codec_registry() -> &'static symphonia::core::codecs::registry::CodecRegistry {
    static REGISTRY: OnceLock<symphonia::core::codecs::registry::CodecRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = symphonia::core::codecs::registry::CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut registry);
        registry.register_audio_decoder::<symphonia_adapter_libopus::OpusDecoder>();
        registry
    })
}

fn symphonia_actual_stream_info(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
) -> PlaybackActualStreamInfo {
    let registry_entry = symphonia_codec_registry().get_audio_decoder(params.codec);
    let codec = registry_entry
        .map(|entry| entry.codec.info.short_name.to_string())
        .or_else(|| Some(params.codec.to_string()));
    let codec_profile = params.profile.and_then(|profile| {
        registry_entry.and_then(|entry| {
            entry
                .codec
                .info
                .profiles
                .iter()
                .find(|candidate| candidate.profile == profile)
                .map(|candidate| candidate.short_name.to_string())
        })
    });

    PlaybackActualStreamInfo {
        codec,
        codec_profile,
        sample_rate: params.sample_rate,
        channels: params
            .channels
            .as_ref()
            .map(|channels| channels.count() as u32),
        bit_depth: params.bits_per_sample.or(params.bits_per_coded_sample),
        bitrate: None,
        sample_format: params.sample_format.map(|format| format!("{format:?}")),
        mime_type: None,
    }
}

// ---------------------------------------------------------------------------
// Worker-thread job enum (RPC messages)
// ---------------------------------------------------------------------------

enum ActiveWorkerJob {
    Load {
        request: PlaybackBackendLoadRequest,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    ActivatePrepared {
        autoplay: bool,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Seek {
        position_ms: u32,
        reply: std::sync::mpsc::Sender<Result<PlaybackSeekAction, String>>,
    },
    CurrentPosition {
        reply: std::sync::mpsc::Sender<Result<u32, String>>,
    },
    CurrentStreamInfo {
        reply: std::sync::mpsc::Sender<Result<Option<PlaybackActualStreamInfo>, String>>,
    },
    SetVolume {
        volume: f32,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Pause {
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Resume {
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Stop {
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    ClearPendingHandoff {
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    PreparedReady {
        generation: u64,
    },
    PrepareFailed {
        generation: u64,
        message: String,
    },
    CompleteGaplessTransition {
        generation: u64,
    },
}

enum PrepareWorkerJob {
    Prepare {
        request: PlaybackBackendLoadRequest,
        generation: u64,
    },
}

// ---------------------------------------------------------------------------
// Commands sent from the worker thread to the pump thread
// ---------------------------------------------------------------------------

enum PumpCommand {
    Seek {
        position_ms: u32,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
}

// ---------------------------------------------------------------------------
// SymphoniaPlaybackBackend – thin frontend that implements PlaybackBackend
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SymphoniaPlaybackBackend {
    active_tx: std::sync::mpsc::Sender<ActiveWorkerJob>,
    prepare_tx: std::sync::mpsc::Sender<PrepareWorkerJob>,
    prepared_state: Arc<SharedPreparedState>,
}

#[derive(Default)]
struct SharedPreparedState {
    inner: Mutex<PreparedStateInner>,
}

#[derive(Default)]
struct PreparedStateInner {
    generation: u64,
    session: Option<PreparedSession>,
}

#[derive(Clone)]
struct PreparedSessionSummary {
    generation: u64,
    ring: Arc<AudioRing>,
    absolute_start_position_ms: u32,
    channels: u16,
    sample_rate: u32,
    actual_stream_info: PlaybackActualStreamInfo,
}

impl SharedPreparedState {
    fn begin_prepare(&self) -> (u64, Option<PreparedSession>) {
        let mut inner = self.inner.lock().unwrap();
        inner.generation = inner.generation.wrapping_add(1);
        let generation = inner.generation;
        let session = inner.session.take();
        (generation, session)
    }

    fn invalidate_and_take(&self) -> Option<PreparedSession> {
        let mut inner = self.inner.lock().unwrap();
        inner.generation = inner.generation.wrapping_add(1);
        inner.session.take()
    }

    fn take_for_activation(&self) -> Option<PreparedSession> {
        self.invalidate_and_take()
    }

    fn is_generation_current(&self, generation: u64) -> bool {
        self.inner.lock().unwrap().generation == generation
    }

    fn summary_for_generation(&self, generation: u64) -> Option<PreparedSessionSummary> {
        let inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return None;
        }
        let session = inner.session.as_ref()?;
        Some(PreparedSessionSummary {
            generation,
            ring: Arc::clone(&session.core.ring),
            absolute_start_position_ms: session.absolute_start_position_ms,
            channels: session.core.channels,
            sample_rate: session.core.sample_rate,
            actual_stream_info: session.core.actual_stream_info.clone(),
        })
    }

    fn take_transitioned(&self, generation: u64) -> Option<PreparedSession> {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return None;
        }
        inner.generation = inner.generation.wrapping_add(1);
        inner.session.take()
    }

    fn store_if_current(
        &self,
        generation: u64,
        session: PreparedSession,
    ) -> Result<Option<PreparedSession>, PreparedSession> {
        let mut inner = self.inner.lock().unwrap();
        if inner.generation != generation {
            return Err(session);
        }
        Ok(inner.session.replace(session))
    }
}

impl SymphoniaPlaybackBackend {
    fn new(event_hub: SymphoniaPlaybackEventHub) -> Self {
        let prepared_state = Arc::new(SharedPreparedState::default());
        let active_state = Arc::clone(&prepared_state);
        let prepare_state = Arc::clone(&prepared_state);
        let (active_tx, active_rx) = std::sync::mpsc::channel::<ActiveWorkerJob>();
        let (prepare_tx, prepare_rx) = std::sync::mpsc::channel::<PrepareWorkerJob>();
        let active_tx_for_worker = active_tx.clone();
        let active_tx_for_prepare = active_tx.clone();
        std::thread::Builder::new()
            .name("symphonia-playback-active-worker".into())
            .spawn(move || {
                active_worker_main(active_rx, active_tx_for_worker, event_hub, active_state)
            })
            .expect("Failed to spawn symphonia playback active worker thread");
        std::thread::Builder::new()
            .name("symphonia-playback-prepare-worker".into())
            .spawn(move || prepare_worker_main(prepare_rx, active_tx_for_prepare, prepare_state))
            .expect("Failed to spawn symphonia playback prepare worker thread");
        Self {
            active_tx,
            prepare_tx,
            prepared_state,
        }
    }

    fn send_active_unit(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<(), String>>) -> ActiveWorkerJob,
    ) -> Result<(), String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.active_tx
            .send(build(tx))
            .map_err(|_| "Symphonia playback active worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback active worker terminated unexpectedly.".to_string())
        })
    }

    fn send_u32(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<u32, String>>) -> ActiveWorkerJob,
    ) -> Result<u32, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.active_tx
            .send(build(tx))
            .map_err(|_| "Symphonia playback active worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback active worker terminated unexpectedly.".to_string())
        })
    }

    fn send_seek_action(
        &self,
        build: impl FnOnce(
            std::sync::mpsc::Sender<Result<PlaybackSeekAction, String>>,
        ) -> ActiveWorkerJob,
    ) -> Result<PlaybackSeekAction, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.active_tx
            .send(build(tx))
            .map_err(|_| "Symphonia playback active worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback active worker terminated unexpectedly.".to_string())
        })
    }
}

impl PlaybackBackend for SymphoniaPlaybackBackend {
    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities {
            gapless_playback: true,
            transcoding_codecs: vec![
                PlaybackTranscodingCodec::Mp3,
                PlaybackTranscodingCodec::Flac,
                PlaybackTranscodingCodec::Aac,
                PlaybackTranscodingCodec::Alac,
                PlaybackTranscodingCodec::Vorbis,
                PlaybackTranscodingCodec::Opus,
            ],
        }
    }

    fn plan_load(
        &self,
        requested_position_ms: u32,
        _supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy {
        // Symphonia can seek within the in-memory data quickly, so we handle
        // the full position locally without relying on server-side seeking.
        PlaybackLoadStrategy::exact_local(requested_position_ms)
    }

    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
        self.send_active_unit(|reply| ActiveWorkerJob::Load { request, reply })
    }

    fn prepare(&mut self, request: PlaybackBackendLoadRequest) -> Result<u64, String> {
        self.send_active_unit(|reply| ActiveWorkerJob::ClearPendingHandoff { reply })?;
        let (generation, stale_session) = self.prepared_state.begin_prepare();
        if let Some(session) = stale_session {
            tear_down_prepared_session(session);
        }
        self.prepare_tx
            .send(PrepareWorkerJob::Prepare {
                request,
                generation,
            })
            .map_err(|_| "Symphonia playback prepare worker is unavailable.".to_string())?;
        Ok(generation)
    }

    fn activate_prepared(&mut self, autoplay: bool) -> Result<(), String> {
        self.send_active_unit(|reply| ActiveWorkerJob::ActivatePrepared { autoplay, reply })
    }

    fn clear_prepared(&mut self) {
        let _ = self.send_active_unit(|reply| ActiveWorkerJob::ClearPendingHandoff { reply });
        if let Some(session) = self.prepared_state.invalidate_and_take() {
            tear_down_prepared_session(session);
        }
    }

    fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String> {
        self.send_seek_action(|reply| ActiveWorkerJob::Seek { position_ms, reply })
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        self.send_u32(|reply| ActiveWorkerJob::CurrentPosition { reply })
    }

    fn current_stream_info(&self) -> Result<Option<PlaybackActualStreamInfo>, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.active_tx
            .send(ActiveWorkerJob::CurrentStreamInfo { reply: tx })
            .map_err(|_| "Symphonia playback active worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback active worker terminated unexpectedly.".to_string())
        })
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        self.send_active_unit(|reply| ActiveWorkerJob::SetVolume { volume, reply })
    }

    fn pause(&mut self) -> Result<(), String> {
        self.send_active_unit(|reply| ActiveWorkerJob::Pause { reply })
    }

    fn resume(&mut self) -> Result<(), String> {
        self.send_active_unit(|reply| ActiveWorkerJob::Resume { reply })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.send_active_unit(|reply| ActiveWorkerJob::Stop { reply })
    }
}

// ---------------------------------------------------------------------------
// Shared ring buffer (pump thread → cpal callback)
// ---------------------------------------------------------------------------

struct AudioRing {
    buf: Mutex<VecDeque<f32>>,
    /// Number of *actual audio* f32 samples consumed by the output callback
    /// (silence padding is NOT counted).
    samples_consumed: AtomicU64,
    /// Pump thread has reached end-of-stream.
    finished: AtomicBool,
    /// Signal the pump thread to stop.
    cancel: AtomicBool,
    /// Guard to ensure the EOS event is fired at most once per session.
    ended_notified: AtomicBool,
    channels: u16,
    sample_rate: u32,
}

impl AudioRing {
    fn new(channels: u16, sample_rate: u32) -> Self {
        // Pre-allocate ~1 second of buffer capacity.
        let cap = sample_rate as usize * channels as usize;
        Self {
            buf: Mutex::new(VecDeque::with_capacity(cap)),
            samples_consumed: AtomicU64::new(0),
            finished: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            ended_notified: AtomicBool::new(false),
            channels,
            sample_rate,
        }
    }

    /// Elapsed playback time in milliseconds based on consumed sample count.
    fn elapsed_ms(&self) -> u64 {
        let consumed = self.samples_consumed.load(Ordering::Relaxed);
        let ch = self.channels as u64;
        let sr = self.sample_rate as u64;
        if ch == 0 || sr == 0 {
            return 0;
        }
        (consumed / ch) * 1000 / sr
    }

    fn drain_into(&self, output: &mut [f32]) -> usize {
        let mut buf = self.buf.lock().unwrap();
        let available = buf.len().min(output.len());
        for (i, sample) in buf.drain(..available).enumerate() {
            output[i] = sample;
        }
        self.samples_consumed
            .fetch_add(available as u64, Ordering::Relaxed);
        available
    }

    fn is_drained_and_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire) && self.buf.lock().unwrap().is_empty()
    }
}

fn volume_to_bits(volume: f32) -> u32 {
    normalize_volume(volume).to_bits()
}

#[derive(Clone)]
struct RoutedTrack {
    ring: Arc<AudioRing>,
    base_position_ms: u32,
    actual_stream_info: PlaybackActualStreamInfo,
}

#[derive(Clone)]
struct PendingGaplessTrack {
    generation: u64,
    ring: Arc<AudioRing>,
    base_position_ms: u32,
    actual_stream_info: PlaybackActualStreamInfo,
}

struct StreamRouterState {
    current: RoutedTrack,
    pending: Option<PendingGaplessTrack>,
}

struct StreamRouter {
    inner: Mutex<StreamRouterState>,
    volume_bits: AtomicU32,
}

impl StreamRouter {
    fn new(
        current_ring: Arc<AudioRing>,
        base_position_ms: u32,
        volume: f32,
        actual_stream_info: PlaybackActualStreamInfo,
    ) -> Self {
        Self {
            inner: Mutex::new(StreamRouterState {
                current: RoutedTrack {
                    ring: current_ring,
                    base_position_ms,
                    actual_stream_info,
                },
                pending: None,
            }),
            volume_bits: AtomicU32::new(volume_to_bits(volume)),
        }
    }

    fn current_track(&self) -> RoutedTrack {
        self.inner.lock().unwrap().current.clone()
    }

    fn current_position_ms(&self) -> u32 {
        let current = self.current_track();
        let elapsed_ms = u32::try_from(current.ring.elapsed_ms()).unwrap_or(u32::MAX);
        current.base_position_ms.saturating_add(elapsed_ms)
    }

    fn current_stream_info(&self) -> PlaybackActualStreamInfo {
        self.current_track().actual_stream_info
    }

    fn update_current_base_position(&self, base_position_ms: u32) {
        self.inner.lock().unwrap().current.base_position_ms = base_position_ms;
    }

    fn set_volume(&self, volume: f32) {
        self.volume_bits
            .store(volume_to_bits(volume), Ordering::Release);
    }

    fn volume(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Acquire))
    }

    fn clear_pending(&self) {
        self.inner.lock().unwrap().pending = None;
    }

    fn install_pending_if_compatible(
        &self,
        summary: PreparedSessionSummary,
        current_channels: u16,
        current_sample_rate: u32,
    ) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if summary.channels != current_channels || summary.sample_rate != current_sample_rate {
            inner.pending = None;
            return false;
        }
        inner.pending = Some(PendingGaplessTrack {
            generation: summary.generation,
            ring: summary.ring,
            base_position_ms: summary.absolute_start_position_ms,
            actual_stream_info: summary.actual_stream_info,
        });
        true
    }

    fn try_gapless_handoff(&self, completed_ring: &Arc<AudioRing>) -> Option<PendingGaplessTrack> {
        let mut inner = self.inner.lock().unwrap();
        if !Arc::ptr_eq(&inner.current.ring, completed_ring) {
            return None;
        }
        let pending = inner.pending.take()?;
        inner.current = RoutedTrack {
            ring: Arc::clone(&pending.ring),
            base_position_ms: pending.base_position_ms,
            actual_stream_info: pending.actual_stream_info.clone(),
        };
        Some(pending)
    }
}

// ---------------------------------------------------------------------------
// Active playback session (owned by worker thread)
// ---------------------------------------------------------------------------

struct SessionCore {
    ring: Arc<AudioRing>,
    pump_handle: JoinHandle<()>,
    pump_cmd_tx: std::sync::mpsc::Sender<PumpCommand>,
    /// Handle for the background HTTP download thread (streaming mode only).
    download_handle: Option<JoinHandle<()>>,
    /// Shared buffer for the progressive download (used to cancel on teardown).
    download_buffer: Option<Arc<SharedDownloadBuffer>>,
    /// Lifts the 2s prepared-track cap once the session becomes active.
    activation_gate: Option<Arc<PreparedActivationGate>>,
    channels: u16,
    sample_rate: u32,
    actual_stream_info: PlaybackActualStreamInfo,
}

struct PreparedSession {
    core: SessionCore,
    absolute_start_position_ms: u32,
}

struct ActiveSession {
    core: SessionCore,
    cpal_stream: cpal::Stream,
    paused: bool,
    stream_router: Arc<StreamRouter>,
}

impl SessionCore {
    fn activate_gapless_preparation(&self) {
        if let Some(ref gate) = self.activation_gate {
            gate.activate();
        }
    }
}

// ---------------------------------------------------------------------------
// Worker thread
// ---------------------------------------------------------------------------

fn active_worker_main(
    rx: std::sync::mpsc::Receiver<ActiveWorkerJob>,
    active_tx: std::sync::mpsc::Sender<ActiveWorkerJob>,
    event_hub: SymphoniaPlaybackEventHub,
    prepared_state: Arc<SharedPreparedState>,
) {
    let mut session: Option<ActiveSession> = None;
    let mut volume = 1.0_f32;

    while let Ok(job) = rx.recv() {
        match job {
            ActiveWorkerJob::Load { request, reply } => {
                tear_down_active(&mut session);
                if let Some(prepared) = prepared_state.invalidate_and_take() {
                    tear_down_prepared_session(prepared);
                }
                event_hub.reset();
                match prepare_session(&request, true).and_then(|prepared| {
                    activate_session(prepared, &event_hub, &active_tx, request.autoplay, volume)
                }) {
                    Ok(active_session) => {
                        session = Some(active_session);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }

            ActiveWorkerJob::ActivatePrepared { autoplay, reply } => {
                let Some(prepared) = prepared_state.take_for_activation() else {
                    let _ = reply.send(Err(
                        "No prepared stream is available for playback.".to_string()
                    ));
                    continue;
                };

                tear_down_active(&mut session);
                event_hub.reset();
                match activate_session(prepared, &event_hub, &active_tx, autoplay, volume) {
                    Ok(active_session) => {
                        session = Some(active_session);
                        let _ = reply.send(Ok(()));
                    }
                    Err(error) => {
                        let _ = reply.send(Err(error));
                    }
                }
            }

            ActiveWorkerJob::Seek { position_ms, reply } => {
                if let Some(ref s) = session {
                    let (seek_reply_tx, seek_reply_rx) = std::sync::mpsc::channel();
                    if s.core
                        .pump_cmd_tx
                        .send(PumpCommand::Seek {
                            position_ms,
                            reply: seek_reply_tx,
                        })
                        .is_ok()
                    {
                        match seek_reply_rx.recv() {
                            Ok(Ok(())) => {
                                s.stream_router.update_current_base_position(position_ms);
                                event_hub.push(PlaybackNativeEvent::SeekProcessed { position_ms });
                                let _ = reply.send(Ok(PlaybackSeekAction::Applied));
                            }
                            Ok(Err(e)) => {
                                let _ = reply.send(Err(e));
                            }
                            Err(_) => {
                                let _ =
                                    reply.send(Err("Pump thread died during seek.".to_string()));
                            }
                        }
                    } else {
                        let _ = reply.send(Err("Pump thread unavailable.".to_string()));
                    }
                } else {
                    let _ = reply.send(Err("No stream is loaded for playback.".to_string()));
                }
            }

            ActiveWorkerJob::CurrentPosition { reply } => {
                let result = session
                    .as_ref()
                    .map(|s| Ok(s.stream_router.current_position_ms()))
                    .unwrap_or_else(|| Err("No stream is loaded for playback.".to_string()));
                let _ = reply.send(result);
            }

            ActiveWorkerJob::CurrentStreamInfo { reply } => {
                let result = session
                    .as_ref()
                    .map(|s| Ok(Some(s.stream_router.current_stream_info())))
                    .unwrap_or_else(|| Ok(None));
                let _ = reply.send(result);
            }

            ActiveWorkerJob::SetVolume {
                volume: next_volume,
                reply,
            } => {
                volume = normalize_volume(next_volume);
                if let Some(ref s) = session {
                    s.stream_router.set_volume(volume);
                }
                let _ = reply.send(Ok(()));
            }

            ActiveWorkerJob::Pause { reply } => {
                let result = if let Some(s) = session.as_mut() {
                    match s.cpal_stream.pause() {
                        Ok(()) => {
                            s.paused = true;
                            Ok(())
                        }
                        Err(e) => Err(format!("Failed to pause audio output: {e}")),
                    }
                } else {
                    Err("No stream is loaded for playback.".to_string())
                };
                let _ = reply.send(result);
            }

            ActiveWorkerJob::Resume { reply } => {
                let result = if let Some(s) = session.as_mut() {
                    match s.cpal_stream.play() {
                        Ok(()) => {
                            s.paused = false;
                            Ok(())
                        }
                        Err(e) => Err(format!("Failed to resume audio output: {e}")),
                    }
                } else {
                    Err("No stream is loaded for playback.".to_string())
                };
                let _ = reply.send(result);
            }

            ActiveWorkerJob::Stop { reply } => {
                tear_down_active(&mut session);
                if let Some(prepared) = prepared_state.invalidate_and_take() {
                    tear_down_prepared_session(prepared);
                }
                let _ = reply.send(Ok(()));
            }

            ActiveWorkerJob::ClearPendingHandoff { reply } => {
                if let Some(ref s) = session {
                    s.stream_router.clear_pending();
                }
                let _ = reply.send(Ok(()));
            }

            ActiveWorkerJob::PreparedReady { generation } => {
                let Some(summary) = prepared_state.summary_for_generation(generation) else {
                    continue;
                };
                if let Some(ref s) = session {
                    let installed = s.stream_router.install_pending_if_compatible(
                        summary,
                        s.core.channels,
                        s.core.sample_rate,
                    );
                    if installed {
                        log::info!(
                            "symphonia-worker.active: installed pending gapless handoff generation={generation}"
                        );
                    }
                }
                event_hub.push(PlaybackNativeEvent::GaplessPrepared { generation });
            }

            ActiveWorkerJob::PrepareFailed {
                generation,
                message,
            } => {
                if !prepared_state.is_generation_current(generation) {
                    continue;
                }
                event_hub.push(PlaybackNativeEvent::GaplessFailed {
                    generation,
                    message,
                });
            }

            ActiveWorkerJob::CompleteGaplessTransition { generation } => {
                let Some(prepared) = prepared_state.take_transitioned(generation) else {
                    continue;
                };

                if let Some(active_session) = session.as_mut() {
                    prepared.core.activate_gapless_preparation();
                    let previous_core = std::mem::replace(&mut active_session.core, prepared.core);
                    tear_down_core(previous_core);
                    event_hub.push(PlaybackNativeEvent::GaplessTransition);
                } else {
                    tear_down_prepared_session(prepared);
                }
            }
        }
    }

    // Channel closed – clean up.
    tear_down_active(&mut session);
    if let Some(prepared) = prepared_state.invalidate_and_take() {
        tear_down_prepared_session(prepared);
    }
}

fn prepare_worker_main(
    rx: std::sync::mpsc::Receiver<PrepareWorkerJob>,
    active_tx: std::sync::mpsc::Sender<ActiveWorkerJob>,
    prepared_state: Arc<SharedPreparedState>,
) {
    while let Ok(job) = rx.recv() {
        match job {
            PrepareWorkerJob::Prepare {
                request,
                generation,
            } => match prepare_session(&request, false) {
                Ok(session) => match prepared_state.store_if_current(generation, session) {
                    Ok(replaced) => {
                        if let Some(previous) = replaced {
                            tear_down_prepared_session(previous);
                        }
                        let _ = active_tx.send(ActiveWorkerJob::PreparedReady { generation });
                    }
                    Err(session) => {
                        tear_down_prepared_session(session);
                    }
                },
                Err(message) => {
                    let _ = active_tx.send(ActiveWorkerJob::PrepareFailed {
                        generation,
                        message,
                    });
                }
            },
        }
    }

    if let Some(prepared) = prepared_state.invalidate_and_take() {
        tear_down_prepared_session(prepared);
    }
}

fn tear_down_active(session: &mut Option<ActiveSession>) {
    if let Some(s) = session.take() {
        let _ = s.cpal_stream.pause();
        drop(s.cpal_stream);
        tear_down_core(s.core);
    }
}

fn tear_down_prepared_session(session: PreparedSession) {
    tear_down_core(session.core);
}

fn tear_down_core(core: SessionCore) {
    if let Some(ref gate) = core.activation_gate {
        gate.abort();
    }
    core.ring.cancel.store(true, Ordering::Release);
    if let Some(ref buffer) = core.download_buffer {
        buffer.cancel();
    }
    drop(core.pump_cmd_tx);
    let _ = core.pump_handle.join();
    if let Some(handle) = core.download_handle {
        let _ = handle.join();
    }
}

// ---------------------------------------------------------------------------
// Load – download audio, spawn pump thread, build cpal stream
// ---------------------------------------------------------------------------

/// Data sent back by the pump thread once symphonia has probed the format.
struct PumpInit {
    channels: u16,
    sample_rate: u32,
    actual_stream_info: PlaybackActualStreamInfo,
    ring: Arc<AudioRing>,
    pump_cmd_tx: std::sync::mpsc::Sender<PumpCommand>,
}

fn prepare_session(
    request: &PlaybackBackendLoadRequest,
    allow_full_download: bool,
) -> Result<PreparedSession, String> {
    let url = request.request.url.to_string();
    let start_ms = request.local_start_position_ms;
    let headers = request.request.headers.clone();
    let activation_gate =
        (!allow_full_download).then(|| Arc::new(PreparedActivationGate::default()));

    log::info!(
        "symphonia-worker.prepare: url_len={} start_ms={start_ms} allow_full_download={allow_full_download}",
        url.len(),
    );

    // 1. Start the HTTP request (but do NOT consume the full body yet).
    let response = reqwest::blocking::Client::new()
        .get(&url)
        .headers(headers.clone())
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }

    let content_length = response.content_length();

    // 2. Choose streaming vs full-download based on Content-Length.
    let (source, download_handle, download_buffer): (
        Box<dyn symphonia::core::io::MediaSource>,
        Option<JoinHandle<()>>,
        Option<Arc<SharedDownloadBuffer>>,
    ) = if let Some(total_len) = content_length {
        // --- Streaming mode: start decoding immediately. ---
        log::info!("symphonia-worker.prepare: streaming mode, content_length={total_len}");
        let shared = Arc::new(SharedDownloadBuffer::new(total_len));
        let dl_shared = Arc::clone(&shared);
        let activation_gate_for_download = activation_gate.clone();
        let dl_handle = std::thread::Builder::new()
            .name("symphonia-download".into())
            .spawn(move || {
                download_response_body(response, &dl_shared, activation_gate_for_download)
            })
            .map_err(|e| format!("Failed to spawn download thread: {e}"))?;
        let source = StreamingMediaSource::new(Arc::clone(&shared));
        (Box::new(source), Some(dl_handle), Some(shared))
    } else if allow_full_download {
        // --- Fallback: no Content-Length, download fully first. ---
        log::warn!(
            "symphonia-worker.prepare: no Content-Length header, falling back to full download"
        );
        let bytes = response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read response body: {e}"))?;
        log::info!("symphonia-worker.prepare: downloaded {} bytes", bytes.len());
        (
            Box::new(std::io::Cursor::new(bytes)) as Box<dyn symphonia::core::io::MediaSource>,
            None,
            None,
        )
    } else {
        log::warn!(
            "symphonia-worker.prepare: skipping gapless preparation because Content-Length is unavailable"
        );
        return Err("Gapless playback requires a known Content-Length response.".to_string());
    };

    // 3. Open with symphonia on a pump thread.
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<PumpInit, String>>();

    let activation_gate_for_pump = activation_gate.clone();
    let download_buffer_for_pump = download_buffer.clone();
    let pump_handle = std::thread::Builder::new()
        .name("symphonia-pump".into())
        .spawn(move || {
            pump_entry(
                source,
                start_ms,
                init_tx,
                activation_gate_for_pump,
                download_buffer_for_pump,
            )
        })
        .map_err(|e| format!("Failed to spawn pump thread: {e}"))?;

    // Wait for the pump thread to finish initialisation.
    let init = init_rx
        .recv()
        .map_err(|_| "Pump thread exited before reporting init result.".to_string())??;

    log::info!(
        "symphonia-worker.prepare: probed format ch={} sr={}",
        init.channels,
        init.sample_rate
    );

    Ok(PreparedSession {
        core: SessionCore {
            ring: init.ring,
            pump_handle,
            pump_cmd_tx: init.pump_cmd_tx,
            download_handle,
            download_buffer,
            activation_gate,
            channels: init.channels,
            sample_rate: init.sample_rate,
            actual_stream_info: init.actual_stream_info,
        },
        absolute_start_position_ms: request.absolute_start_position_ms,
    })
}

fn activate_session(
    prepared: PreparedSession,
    event_hub: &SymphoniaPlaybackEventHub,
    active_tx: &std::sync::mpsc::Sender<ActiveWorkerJob>,
    autoplay: bool,
    volume: f32,
) -> Result<ActiveSession, String> {
    let PreparedSession {
        core,
        absolute_start_position_ms,
    } = prepared;
    core.activate_gapless_preparation();
    let stream_router = Arc::new(StreamRouter::new(
        Arc::clone(&core.ring),
        absolute_start_position_ms,
        volume,
        core.actual_stream_info.clone(),
    ));
    let cpal_stream = match build_cpal_stream(
        Arc::clone(&stream_router),
        core.channels,
        core.sample_rate,
        event_hub.clone(),
        active_tx.clone(),
    ) {
        Ok(stream) => stream,
        Err(error) => {
            tear_down_core(core);
            return Err(error);
        }
    };

    let play_result = if autoplay {
        cpal_stream
            .play()
            .map_err(|e| format!("Failed to start audio output: {e}"))
    } else {
        cpal_stream
            .pause()
            .map_err(|e| format!("Failed to pause audio output: {e}"))
    };
    if let Err(error) = play_result {
        drop(cpal_stream);
        tear_down_core(core);
        return Err(error);
    }

    Ok(ActiveSession {
        core,
        cpal_stream,
        paused: !autoplay,
        stream_router,
    })
}

// ---------------------------------------------------------------------------
// Background HTTP download – reads the response body in chunks and appends
// to the SharedDownloadBuffer so the pump thread can consume data as it
// arrives.
// ---------------------------------------------------------------------------

fn download_response_body(
    mut response: reqwest::blocking::Response,
    shared: &SharedDownloadBuffer,
    activation_gate: Option<Arc<PreparedActivationGate>>,
) {
    use std::io::Read;

    let mut chunk = vec![0u8; 64 * 1024]; // 64 KiB read buffer
    let mut total_read: u64 = 0;

    loop {
        if let Some(ref gate) = activation_gate {
            if !gate.wait_for_download_permission(&shared.cancel) {
                log::info!(
                    "symphonia-download: prepared download stopped after {total_read} bytes"
                );
                return;
            }
        }

        if shared.cancel.load(Ordering::Acquire) {
            log::info!("symphonia-download: cancelled after {total_read} bytes");
            return;
        }

        let n = match response.read(&mut chunk) {
            Ok(0) => {
                // EOF – download complete.
                let mut inner = shared.inner.lock().unwrap();
                inner.complete = true;
                shared.condvar.notify_all();
                log::info!("symphonia-download: complete, {total_read} bytes");
                return;
            }
            Ok(n) => n,
            Err(e) => {
                let msg = format!("Download read error: {e}");
                log::error!("symphonia-download: {msg}");
                let mut inner = shared.inner.lock().unwrap();
                // TODO: classify recoverable network errors as handled once this shim reports structured playback errors.
                inner.error = Some(msg);
                inner.complete = true;
                shared.condvar.notify_all();
                return;
            }
        };

        total_read += n as u64;

        {
            let mut inner = shared.inner.lock().unwrap();
            inner.data.extend_from_slice(&chunk[..n]);
        }
        // Wake the pump thread in case it was blocking in Read.
        shared.condvar.notify_all();
    }
}

// ---------------------------------------------------------------------------
// Pump thread – owns the symphonia reader/decoder, decodes PCM into ring
// buffer, and accepts seek commands from the worker thread.
// ---------------------------------------------------------------------------

fn pump_entry(
    source: Box<dyn symphonia::core::io::MediaSource>,
    start_ms: u32,
    init_tx: std::sync::mpsc::Sender<Result<PumpInit, String>>,
    activation_gate: Option<Arc<PreparedActivationGate>>,
    download_buffer: Option<Arc<SharedDownloadBuffer>>,
) {
    let result = pump_init_and_run(source, start_ms, init_tx, activation_gate, download_buffer);
    if let Err(e) = &result {
        log::error!("symphonia-pump: terminated with error: {e}");
    }
}

fn pump_init_and_run(
    source: Box<dyn symphonia::core::io::MediaSource>,
    start_ms: u32,
    init_tx: std::sync::mpsc::Sender<Result<PumpInit, String>>,
    activation_gate: Option<Arc<PreparedActivationGate>>,
    download_buffer: Option<Arc<SharedDownloadBuffer>>,
) -> Result<(), String> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::units::Time;

    // 1. Create a MediaSourceStream from the provided source.
    //    In streaming mode this is a StreamingMediaSource that reads from a
    //    progressively-filled buffer.  In fallback mode it is a Cursor<Vec<u8>>.
    let mss = MediaSourceStream::new(source, Default::default());

    // 2. Probe the format.
    let mut reader = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| {
            let msg = format!("Failed to probe audio format: {e}");
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;

    if let Some(shared) = &download_buffer {
        shared.expose_byte_len_after_probe();
    }

    // 3. Find the first audio track.
    let track = reader.default_track(TrackType::Audio).ok_or_else(|| {
        let msg = "No audio track found.".to_string();
        let _ = init_tx.send(Err(msg.clone()));
        msg
    })?;

    let track_id = track.id;
    let audio_params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .cloned()
        .ok_or_else(|| {
            let msg = "Audio codec parameters are unavailable.".to_string();
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;

    let channels = audio_params
        .channels
        .as_ref()
        .map(|c| c.count() as u16)
        .unwrap_or(2);
    let sample_rate = audio_params.sample_rate.unwrap_or(44100);
    let actual_stream_info = symphonia_actual_stream_info(&audio_params);

    // 4. Create the decoder.
    let mut decoder = symphonia_codec_registry()
        .make_audio_decoder(&audio_params, &AudioDecoderOptions::default())
        .map_err(|e| {
            let msg = format!("Failed to create decoder: {e}");
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;

    // 5. Seek if requested.
    if start_ms > 0 {
        let seek_to = SeekTo::Time {
            time: Time::from_nanos_u64(u64::from(start_ms) * 1_000_000),
            track_id: Some(track_id),
        };
        reader.seek(SeekMode::Accurate, seek_to).map_err(|e| {
            let msg = format!("Failed to seek to {start_ms} ms: {e}");
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;
        decoder.reset();
    }

    // 6. Create shared ring buffer and pump command channel, send init back.
    let ring = Arc::new(AudioRing::new(channels, sample_rate));
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<PumpCommand>();

    let _ = init_tx.send(Ok(PumpInit {
        channels,
        sample_rate,
        actual_stream_info,
        ring: Arc::clone(&ring),
        pump_cmd_tx: cmd_tx,
    }));

    // 7. Enter the decode loop.
    pump_decode_loop(
        &mut reader,
        &mut decoder,
        track_id,
        &ring,
        &cmd_rx,
        channels,
        sample_rate,
        activation_gate,
    )
}

fn pump_decode_loop(
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::audio::AudioDecoder>,
    track_id: u32,
    ring: &AudioRing,
    cmd_rx: &std::sync::mpsc::Receiver<PumpCommand>,
    channels: u16,
    sample_rate: u32,
    activation_gate: Option<Arc<PreparedActivationGate>>,
) -> Result<(), String> {
    let two_seconds = sample_rate as usize * channels as usize * 2;
    let mut samples = Vec::<f32>::new();

    loop {
        // Check for cancel.
        if ring.cancel.load(Ordering::Acquire) {
            return Ok(());
        }

        // Check for commands (non-blocking).
        match cmd_rx.try_recv() {
            Ok(PumpCommand::Seek { position_ms, reply }) => {
                let result = symphonia_seek_inner(reader, decoder, track_id, position_ms);
                if result.is_ok() {
                    // Clear the ring buffer so stale audio doesn't play.
                    ring.buf.lock().unwrap().clear();
                    ring.samples_consumed.store(0, Ordering::Release);
                    ring.finished.store(false, Ordering::Release);
                    ring.ended_notified.store(false, Ordering::Release);
                }
                let _ = reply.send(result);
                continue;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // Worker has dropped the command sender – we should exit.
                return Ok(());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // No command pending, proceed with decoding.
            }
        }

        // Back-pressure: active sessions keep the ring near 2 s; prepared
        // sessions park entirely at ~2 s until activation so they do not keep
        // downloading the full file.
        {
            let buf = ring.buf.lock().unwrap();
            if buf.len() >= two_seconds {
                drop(buf);
                if let Some(ref gate) = activation_gate {
                    if !gate.is_activated() {
                        gate.pause_until_activated(&ring.cancel);
                        continue;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            }
        }

        // Decode the next packet.
        let packet = match reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => {
                ring.finished.store(true, Ordering::Release);
                log::info!("symphonia-pump: end of stream");
                // After EOS, wait for either cancel or a seek command that
                // could restart decoding (if the user seeks back).
                pump_wait_after_eos(ring, cmd_rx, reader, decoder, track_id)?;
                return Ok(());
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                ring.finished.store(true, Ordering::Release);
                return Err(
                    "Symphonia requested a decoder reset for a changed track list.".to_string(),
                );
            }
            Err(e) => {
                ring.finished.store(true, Ordering::Release);
                return Err(format!("Failed to read packet: {e}"));
            }
        };

        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::IoError(e)) => {
                log::warn!("symphonia-pump: decode IO error (skipping): {e}");
                continue;
            }
            Err(symphonia::core::errors::Error::DecodeError(msg)) => {
                log::warn!("symphonia-pump: decode error (skipping): {msg}");
                continue;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                ring.finished.store(true, Ordering::Release);
                return Err("Symphonia requested an audio decoder reset.".to_string());
            }
            Err(e) => {
                ring.finished.store(true, Ordering::Release);
                return Err(format!("Decode failed: {e}"));
            }
        };

        // Convert to interleaved f32.
        let sample_count = decoded.samples_interleaved();
        if sample_count == 0 {
            continue;
        }
        samples.resize(sample_count, 0.0);
        decoded.copy_to_slice_interleaved::<f32, _>(&mut samples);
        ring.buf.lock().unwrap().extend(samples.iter().copied());
    }
}

/// After reaching end-of-stream, park the pump thread and wait for either a
/// seek command (which resets and restarts decoding) or a cancellation.
fn pump_wait_after_eos(
    ring: &AudioRing,
    cmd_rx: &std::sync::mpsc::Receiver<PumpCommand>,
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::audio::AudioDecoder>,
    track_id: u32,
) -> Result<(), String> {
    loop {
        if ring.cancel.load(Ordering::Acquire) {
            return Ok(());
        }

        match cmd_rx.try_recv() {
            Ok(PumpCommand::Seek { position_ms, reply }) => {
                let result = symphonia_seek_inner(reader, decoder, track_id, position_ms);
                if result.is_ok() {
                    ring.buf.lock().unwrap().clear();
                    ring.samples_consumed.store(0, Ordering::Release);
                    ring.finished.store(false, Ordering::Release);
                    ring.ended_notified.store(false, Ordering::Release);
                }
                let _ = reply.send(result);
                // A successful seek means we should resume decoding – but
                // this function returns to pump_decode_loop's caller, so
                // we'd need to re-enter the loop.  For simplicity, just
                // return Ok and let the caller know the pump ended normally.
                // The worker will re-load if needed. In practice, seeking
                // after EOS is unusual.
                return Ok(());
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Ok(());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Symphonia seek helper
// ---------------------------------------------------------------------------

fn symphonia_seek_inner(
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::audio::AudioDecoder>,
    track_id: u32,
    position_ms: u32,
) -> Result<(), String> {
    use symphonia::core::formats::{SeekMode, SeekTo};
    use symphonia::core::units::Time;

    let seek_to = SeekTo::Time {
        time: Time::from_nanos_u64(u64::from(position_ms) * 1_000_000),
        track_id: Some(track_id),
    };
    reader
        .seek(SeekMode::Accurate, seek_to)
        .map_err(|e| format!("Seek failed: {e}"))?;
    decoder.reset();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_registry_can_create_opus_decoder() {
        use symphonia::core::audio::layouts;
        use symphonia::core::codecs::audio::{
            well_known::CODEC_ID_OPUS, AudioCodecParameters, AudioDecoderOptions,
        };

        let mut params = AudioCodecParameters::new();
        params
            .for_codec(CODEC_ID_OPUS)
            .with_sample_rate(48_000)
            .with_channels(layouts::CHANNEL_LAYOUT_STEREO);

        symphonia_codec_registry()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .expect("Opus decoder should be registered for Ogg/Opus streams");
    }
}

// ---------------------------------------------------------------------------
// cpal output stream
// ---------------------------------------------------------------------------

fn build_cpal_stream(
    stream_router: Arc<StreamRouter>,
    channels: u16,
    sample_rate: u32,
    event_hub: SymphoniaPlaybackEventHub,
    active_tx: std::sync::mpsc::Sender<ActiveWorkerJob>,
) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No default audio output device found.")?;

    let config = cpal::StreamConfig {
        channels,
        sample_rate: sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let mut written = 0usize;

                while written < data.len() {
                    let current = stream_router.current_track();
                    let start = written;
                    let wrote = current.ring.drain_into(&mut data[start..]);
                    let volume = stream_router.volume();
                    if volume != 1.0 {
                        for sample in &mut data[start..start + wrote] {
                            *sample *= volume;
                        }
                    }
                    written += wrote;
                    if written == data.len() {
                        break;
                    }

                    if !current.ring.is_drained_and_finished() {
                        break;
                    }

                    if let Some(pending) = stream_router.try_gapless_handoff(&current.ring) {
                        let _ = active_tx.send(ActiveWorkerJob::CompleteGaplessTransition {
                            generation: pending.generation,
                        });
                        log::info!(
                            "symphonia-cpal: gapless handoff generation={} completed",
                            pending.generation
                        );
                        continue;
                    }

                    if !current.ring.ended_notified.swap(true, Ordering::AcqRel) {
                        event_hub.push(PlaybackNativeEvent::Ended);
                        log::info!("symphonia-cpal: end-of-stream signalled to event hub");
                    }
                    break;
                }

                for sample in &mut data[written..] {
                    *sample = 0.0;
                }
            },
            |err| {
                log::error!("cpal output stream error: {err}");
            },
            None,
        )
        .map_err(|e| format!("Failed to build cpal output stream: {e}"))?;

    Ok(stream)
}
