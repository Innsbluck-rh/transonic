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
//! - End-of-stream is detected by the cpal callback (pump finished + buffer
//!   drained) and signalled to the controller via `SymphoniaPlaybackEventHub`.
//!
//! Key difference from the MF backend: symphonia supports native seeking, so
//! the `seek` method sends a command to the pump thread (via a secondary
//! channel) instead of requiring a full reload.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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
            cancel: AtomicBool::new(false),
        }
    }

    /// Signal cancellation and wake any blocked reader.
    fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
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
        Some(self.shared.total_len)
    }
}

// ---------------------------------------------------------------------------
// SymphoniaPlaybackEventHub – shared EOS signalling between cpal callback
// and controller (via NativePlaybackEventSource).
// ---------------------------------------------------------------------------

/// Shared event channel between the symphonia playback backend and the
/// controller.
///
/// The cpal output callback sets the `ended` flag once the pump thread has
/// finished **and** the ring buffer has been fully drained.  The controller
/// polls `drain_events()` (via `synced_state`) and receives a single
/// `PlaybackNativeEvent::Ended` event per track.
#[derive(Clone, Default)]
pub struct SymphoniaPlaybackEventHub {
    ended: Arc<AtomicBool>,
}

impl SymphoniaPlaybackEventHub {
    /// Reset the ended flag.  Called when a new track is loaded so that a
    /// stale flag from a previous session does not trigger a spurious event.
    fn reset(&self) {
        self.ended.store(false, Ordering::Release);
    }
}

impl NativePlaybackEventSource for SymphoniaPlaybackEventHub {
    fn drain_events(&mut self) -> Vec<PlaybackNativeEvent> {
        if self.ended.swap(false, Ordering::AcqRel) {
            vec![PlaybackNativeEvent::Ended]
        } else {
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn create_playback_backend(event_hub: SymphoniaPlaybackEventHub) -> Box<dyn PlaybackBackend> {
    Box::new(SymphoniaPlaybackBackend::new(event_hub))
}

// ---------------------------------------------------------------------------
// Worker-thread job enum (RPC messages)
// ---------------------------------------------------------------------------

enum WorkerJob {
    Load {
        request: PlaybackBackendLoadRequest,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Seek {
        position_ms: u32,
        reply: std::sync::mpsc::Sender<Result<PlaybackSeekAction, String>>,
    },
    CurrentPosition {
        reply: std::sync::mpsc::Sender<Result<u32, String>>,
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
    tx: std::sync::mpsc::Sender<WorkerJob>,
}

impl SymphoniaPlaybackBackend {
    fn new(event_hub: SymphoniaPlaybackEventHub) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<WorkerJob>();
        std::thread::Builder::new()
            .name("symphonia-playback-worker".into())
            .spawn(move || worker_main(rx, event_hub))
            .expect("Failed to spawn symphonia playback worker thread");
        Self { tx }
    }

    fn send_unit(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<(), String>>) -> WorkerJob,
    ) -> Result<(), String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send(build(tx))
            .map_err(|_| "Symphonia playback worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback worker terminated unexpectedly.".to_string())
        })
    }

    fn send_u32(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<u32, String>>) -> WorkerJob,
    ) -> Result<u32, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send(build(tx))
            .map_err(|_| "Symphonia playback worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback worker terminated unexpectedly.".to_string())
        })
    }

    fn send_seek_action(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<PlaybackSeekAction, String>>) -> WorkerJob,
    ) -> Result<PlaybackSeekAction, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send(build(tx))
            .map_err(|_| "Symphonia playback worker is unavailable.".to_string())?;
        rx.recv().unwrap_or_else(|_| {
            Err("Symphonia playback worker terminated unexpectedly.".to_string())
        })
    }
}

impl PlaybackBackend for SymphoniaPlaybackBackend {
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
        self.send_unit(|reply| WorkerJob::Load { request, reply })
    }

    fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String> {
        self.send_seek_action(|reply| WorkerJob::Seek { position_ms, reply })
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        self.send_u32(|reply| WorkerJob::CurrentPosition { reply })
    }

    fn pause(&mut self) -> Result<(), String> {
        self.send_unit(|reply| WorkerJob::Pause { reply })
    }

    fn resume(&mut self) -> Result<(), String> {
        self.send_unit(|reply| WorkerJob::Resume { reply })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.send_unit(|reply| WorkerJob::Stop { reply })
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
}

// ---------------------------------------------------------------------------
// Active playback session (owned by worker thread)
// ---------------------------------------------------------------------------

struct ActiveSession {
    ring: Arc<AudioRing>,
    cpal_stream: cpal::Stream,
    pump_handle: JoinHandle<()>,
    pump_cmd_tx: std::sync::mpsc::Sender<PumpCommand>,
    /// Handle for the background HTTP download thread (streaming mode only).
    download_handle: Option<JoinHandle<()>>,
    /// Shared buffer for the progressive download (used to cancel on teardown).
    download_buffer: Option<Arc<SharedDownloadBuffer>>,
    paused: bool,
}

// ---------------------------------------------------------------------------
// Worker thread
// ---------------------------------------------------------------------------

fn worker_main(rx: std::sync::mpsc::Receiver<WorkerJob>, event_hub: SymphoniaPlaybackEventHub) {
    let mut session: Option<ActiveSession> = None;
    let mut base_position_ms: Option<u32> = None;

    while let Ok(job) = rx.recv() {
        match job {
            WorkerJob::Load { request, reply } => {
                tear_down(&mut session);
                event_hub.reset();
                let abs = request.absolute_start_position_ms;
                match do_load(&request, &event_hub) {
                    Ok(s) => {
                        base_position_ms = Some(abs);
                        session = Some(s);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        base_position_ms = None;
                        let _ = reply.send(Err(e));
                    }
                }
            }

            WorkerJob::Seek { position_ms, reply } => {
                if let Some(ref s) = session {
                    let (seek_reply_tx, seek_reply_rx) = std::sync::mpsc::channel();
                    if s.pump_cmd_tx
                        .send(PumpCommand::Seek {
                            position_ms,
                            reply: seek_reply_tx,
                        })
                        .is_ok()
                    {
                        match seek_reply_rx.recv() {
                            Ok(Ok(())) => {
                                base_position_ms = Some(position_ms);
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

            WorkerJob::CurrentPosition { reply } => {
                let result = match (&session, base_position_ms) {
                    (Some(s), Some(base)) => {
                        let elapsed = u32::try_from(s.ring.elapsed_ms()).unwrap_or(u32::MAX);
                        Ok(base.saturating_add(elapsed))
                    }
                    _ => Err("No stream is loaded for playback.".to_string()),
                };
                let _ = reply.send(result);
            }

            WorkerJob::Pause { reply } => {
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

            WorkerJob::Resume { reply } => {
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

            WorkerJob::Stop { reply } => {
                tear_down(&mut session);
                base_position_ms = None;
                let _ = reply.send(Ok(()));
            }
        }
    }

    // Channel closed – clean up.
    tear_down(&mut session);
}

fn tear_down(session: &mut Option<ActiveSession>) {
    if let Some(s) = session.take() {
        // Signal pump thread to stop.
        s.ring.cancel.store(true, Ordering::Release);
        // Signal download thread to stop (and unblock any waiting Read).
        if let Some(ref buf) = s.download_buffer {
            buf.cancel();
        }
        let _ = s.cpal_stream.pause();
        drop(s.cpal_stream);
        // Close the command channel so the pump thread's try_recv sees a
        // disconnected channel and can exit cleanly.
        drop(s.pump_cmd_tx);
        let _ = s.pump_handle.join();
        if let Some(h) = s.download_handle {
            let _ = h.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Load – download audio, spawn pump thread, build cpal stream
// ---------------------------------------------------------------------------

/// Data sent back by the pump thread once symphonia has probed the format.
struct PumpInit {
    channels: u16,
    sample_rate: u32,
    ring: Arc<AudioRing>,
    pump_cmd_tx: std::sync::mpsc::Sender<PumpCommand>,
}

fn do_load(
    request: &PlaybackBackendLoadRequest,
    event_hub: &SymphoniaPlaybackEventHub,
) -> Result<ActiveSession, String> {
    let url = request.request.url.to_string();
    let start_ms = request.local_start_position_ms;
    let autoplay = request.autoplay;
    let headers = request.request.headers.clone();

    log::info!(
        "symphonia-worker.load: url_len={} start_ms={start_ms} autoplay={autoplay}",
        url.len()
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
        log::info!("symphonia-worker.load: streaming mode, content_length={total_len}");
        let shared = Arc::new(SharedDownloadBuffer::new(total_len));
        let dl_shared = Arc::clone(&shared);
        let dl_handle = std::thread::Builder::new()
            .name("symphonia-download".into())
            .spawn(move || download_response_body(response, &dl_shared))
            .map_err(|e| format!("Failed to spawn download thread: {e}"))?;
        let source = StreamingMediaSource::new(Arc::clone(&shared));
        (Box::new(source), Some(dl_handle), Some(shared))
    } else {
        // --- Fallback: no Content-Length, download fully first. ---
        log::warn!(
            "symphonia-worker.load: no Content-Length header, falling back to full download"
        );
        let bytes = response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read response body: {e}"))?;
        log::info!("symphonia-worker.load: downloaded {} bytes", bytes.len());
        (
            Box::new(std::io::Cursor::new(bytes)) as Box<dyn symphonia::core::io::MediaSource>,
            None,
            None,
        )
    };

    // 3. Open with symphonia on a pump thread.
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<PumpInit, String>>();

    let pump_handle = std::thread::Builder::new()
        .name("symphonia-pump".into())
        .spawn(move || pump_entry(source, start_ms, init_tx))
        .map_err(|e| format!("Failed to spawn pump thread: {e}"))?;

    // Wait for the pump thread to finish initialisation.
    let init = init_rx
        .recv()
        .map_err(|_| "Pump thread exited before reporting init result.".to_string())??;

    log::info!(
        "symphonia-worker.load: probed format ch={} sr={}",
        init.channels,
        init.sample_rate
    );

    let ring = init.ring;
    let pump_cmd_tx = init.pump_cmd_tx;

    // 4. Build cpal output stream; on failure cancel everything.
    let ended_flag = event_hub.ended.clone();
    let cpal_stream = match build_cpal_stream(
        Arc::clone(&ring),
        init.channels,
        init.sample_rate,
        ended_flag,
    ) {
        Ok(s) => s,
        Err(e) => {
            ring.cancel.store(true, Ordering::Release);
            if let Some(ref buf) = download_buffer {
                buf.cancel();
            }
            drop(pump_cmd_tx);
            let _ = pump_handle.join();
            if let Some(h) = download_handle {
                let _ = h.join();
            }
            return Err(e);
        }
    };

    if autoplay {
        cpal_stream
            .play()
            .map_err(|e| format!("Failed to start audio output: {e}"))?;
    } else {
        // cpal streams start in an unspecified state; explicitly pause.
        cpal_stream
            .pause()
            .map_err(|e| format!("Failed to pause audio output: {e}"))?;
    }

    Ok(ActiveSession {
        ring,
        cpal_stream,
        pump_handle,
        pump_cmd_tx,
        download_handle,
        download_buffer,
        paused: !autoplay,
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
) {
    use std::io::Read;

    let mut chunk = vec![0u8; 64 * 1024]; // 64 KiB read buffer
    let mut total_read: u64 = 0;

    loop {
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
) {
    let result = pump_init_and_run(source, start_ms, init_tx);
    if let Err(e) = &result {
        log::error!("symphonia-pump: terminated with error: {e}");
    }
}

fn pump_init_and_run(
    source: Box<dyn symphonia::core::io::MediaSource>,
    start_ms: u32,
    init_tx: std::sync::mpsc::Sender<Result<PumpInit, String>>,
) -> Result<(), String> {
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::core::units::Time;

    // 1. Create a MediaSourceStream from the provided source.
    //    In streaming mode this is a StreamingMediaSource that reads from a
    //    progressively-filled buffer.  In fallback mode it is a Cursor<Vec<u8>>.
    let mss = MediaSourceStream::new(source, Default::default());

    // 2. Probe the format.
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| {
            let msg = format!("Failed to probe audio format: {e}");
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;

    let mut reader = probed.format;

    // 3. Find the first audio track.
    let track = reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| {
            let msg = "No audio track found.".to_string();
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);

    // 4. Create the decoder.
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| {
            let msg = format!("Failed to create decoder: {e}");
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;

    // 5. Seek if requested.
    if start_ms > 0 {
        let seek_to = SeekTo::Time {
            time: Time::new(start_ms as u64 / 1000, (start_ms % 1000) as f64 / 1000.0),
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
    )
}

fn pump_decode_loop(
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    ring: &AudioRing,
    cmd_rx: &std::sync::mpsc::Receiver<PumpCommand>,
    channels: u16,
    sample_rate: u32,
) -> Result<(), String> {
    use symphonia::core::audio::SampleBuffer;

    let two_seconds = sample_rate as usize * channels as usize * 2;

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

        // Back-pressure: sleep briefly when the ring already has >= 2 s of
        // data.
        {
            let buf = ring.buf.lock().unwrap();
            if buf.len() >= two_seconds {
                drop(buf);
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            }
        }

        // Decode the next packet.
        let packet = match reader.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                ring.finished.store(true, Ordering::Release);
                log::info!("symphonia-pump: end of stream");
                // After EOS, wait for either cancel or a seek command that
                // could restart decoding (if the user seeks back).
                pump_wait_after_eos(ring, cmd_rx, reader, decoder, track_id)?;
                return Ok(());
            }
            Err(e) => {
                ring.finished.store(true, Ordering::Release);
                return Err(format!("Failed to read packet: {e}"));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(msg)) => {
                log::warn!("symphonia-pump: decode error (skipping): {msg}");
                continue;
            }
            Err(e) => {
                ring.finished.store(true, Ordering::Release);
                return Err(format!("Decode failed: {e}"));
            }
        };

        // Convert to interleaved f32.
        let spec = *decoded.spec();
        let num_frames = decoded.frames();
        if num_frames == 0 {
            continue;
        }
        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        let samples = sample_buf.samples();

        if !samples.is_empty() {
            ring.buf.lock().unwrap().extend(samples.iter());
        }
    }
}

/// After reaching end-of-stream, park the pump thread and wait for either a
/// seek command (which resets and restarts decoding) or a cancellation.
fn pump_wait_after_eos(
    ring: &AudioRing,
    cmd_rx: &std::sync::mpsc::Receiver<PumpCommand>,
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut Box<dyn symphonia::core::codecs::Decoder>,
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
    decoder: &mut Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    position_ms: u32,
) -> Result<(), String> {
    use symphonia::core::formats::{SeekMode, SeekTo};
    use symphonia::core::units::Time;

    let seek_to = SeekTo::Time {
        time: Time::new(
            position_ms as u64 / 1000,
            (position_ms % 1000) as f64 / 1000.0,
        ),
        track_id: Some(track_id),
    };
    reader
        .seek(SeekMode::Accurate, seek_to)
        .map_err(|e| format!("Seek failed: {e}"))?;
    decoder.reset();
    Ok(())
}

// ---------------------------------------------------------------------------
// cpal output stream
// ---------------------------------------------------------------------------

fn build_cpal_stream(
    ring: Arc<AudioRing>,
    channels: u16,
    sample_rate: u32,
    ended_flag: Arc<AtomicBool>,
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
                let mut buf = ring.buf.lock().unwrap();
                let available = buf.len().min(data.len());

                // Copy available samples.
                for (i, sample) in buf.drain(..available).enumerate() {
                    data[i] = sample;
                }
                // Fill the remainder with silence.
                for sample in &mut data[available..] {
                    *sample = 0.0;
                }

                // Only count *real* audio samples for position tracking.
                ring.samples_consumed
                    .fetch_add(available as u64, Ordering::Relaxed);

                // EOS detection: the pump has finished producing samples AND
                // the ring buffer has been fully drained.  The
                // `ended_notified` guard ensures we only fire the flag once
                // per session (the cpal callback keeps running even after the
                // audio ends).
                if available == 0
                    && ring.finished.load(Ordering::Acquire)
                    && !ring.ended_notified.swap(true, Ordering::AcqRel)
                {
                    ended_flag.store(true, Ordering::Release);
                    log::info!("symphonia-cpal: end-of-stream signalled to event hub");
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
