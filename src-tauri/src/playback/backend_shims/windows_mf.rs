//! Windows playback backend using Media Foundation (IMFSourceReader) for decoding
//! and cpal for audio output.
//!
//! Architecture:
//! - `MfPlaybackBackend` sends commands to a dedicated worker thread via mpsc.
//! - On load the worker spawns a *pump thread* that creates an `IMFSourceReader`
//!   from the HTTP URL (MF handles streaming/buffering internally) and reads
//!   decoded f32 PCM samples into a shared ring buffer.
//! - A cpal output stream drains the ring buffer to produce audio.
//! - End-of-stream is detected by the cpal callback (pump finished + buffer
//!   drained) and signalled to the controller via `MfPlaybackEventHub`.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use windows::core::{GUID, HSTRING};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::System::Variant::VT_I8;

use crate::models::{PlaybackCapabilities, PlaybackTranscodingCodec};
use crate::playback::backend_shims::backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};
use crate::playback::native_events::{NativePlaybackEventSource, PlaybackNativeEvent};

// ---------------------------------------------------------------------------
// Constants – defined locally to avoid type-conversion issues across
// different windows-crate versions.
// ---------------------------------------------------------------------------

/// `MF_SOURCE_READER_FIRST_AUDIO_STREAM` (0xFFFF_FFFD)
const FIRST_AUDIO_STREAM: u32 = 0xFFFF_FFFD;

/// `MF_SOURCE_READERF_ERROR` flag (0x1)
const READERF_ERROR: u32 = 0x0000_0001;

/// `MF_SOURCE_READERF_ENDOFSTREAM` flag (0x2)
const READERF_END_OF_STREAM: u32 = 0x0000_0002;

// ---------------------------------------------------------------------------
// MfPlaybackEventHub – shared EOS signalling between cpal callback and
// controller (via NativePlaybackEventSource).
// ---------------------------------------------------------------------------

/// Shared event channel between the MF playback backend and the controller.
///
/// The cpal output callback sets the `ended` flag once the pump thread has
/// finished **and** the ring buffer has been fully drained.  The controller
/// polls `drain_events()` (via `synced_state`) and receives a single
/// `PlaybackNativeEvent::Ended` event per track.
#[derive(Clone, Default)]
pub struct MfPlaybackEventHub {
    ended: Arc<AtomicBool>,
}

impl MfPlaybackEventHub {
    /// Reset the ended flag.  Called when a new track is loaded so that a
    /// stale flag from a previous session does not trigger a spurious event.
    fn reset(&self) {
        self.ended.store(false, Ordering::Release);
    }
}

impl NativePlaybackEventSource for MfPlaybackEventHub {
    fn drain_events(&mut self) -> Vec<PlaybackNativeEvent> {
        // Atomically swap: if `ended` was true, consume it and emit the event
        // exactly once.
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

pub fn create_playback_backend(event_hub: MfPlaybackEventHub) -> Box<dyn PlaybackBackend> {
    Box::new(MfPlaybackBackend::new(event_hub))
}

// ---------------------------------------------------------------------------
// Worker-thread job enum (RPC messages)
// ---------------------------------------------------------------------------

enum WorkerJob {
    Load {
        request: PlaybackBackendLoadRequest,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
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
// MfPlaybackBackend – thin frontend that implements PlaybackBackend
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MfPlaybackBackend {
    tx: std::sync::mpsc::Sender<WorkerJob>,
}

impl MfPlaybackBackend {
    fn new(event_hub: MfPlaybackEventHub) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<WorkerJob>();
        std::thread::Builder::new()
            .name("mf-playback-worker".into())
            .spawn(move || worker_main(rx, event_hub))
            .expect("Failed to spawn MF playback worker thread");
        Self { tx }
    }

    fn send_unit(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<(), String>>) -> WorkerJob,
    ) -> Result<(), String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send(build(tx))
            .map_err(|_| "MF playback worker is unavailable.".to_string())?;
        rx.recv()
            .unwrap_or_else(|_| Err("MF playback worker terminated unexpectedly.".to_string()))
    }

    fn send_u32(
        &self,
        build: impl FnOnce(std::sync::mpsc::Sender<Result<u32, String>>) -> WorkerJob,
    ) -> Result<u32, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send(build(tx))
            .map_err(|_| "MF playback worker is unavailable.".to_string())?;
        rx.recv()
            .unwrap_or_else(|_| Err("MF playback worker terminated unexpectedly.".to_string()))
    }
}

impl PlaybackBackend for MfPlaybackBackend {
    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities {
            gapless_playback: false,
            transcoding_codecs: vec![
                PlaybackTranscodingCodec::Mp3,
                PlaybackTranscodingCodec::Aac,
                PlaybackTranscodingCodec::Flac,
            ],
            output_device_selection: false,
        }
    }

    fn plan_load(
        &self,
        requested_position_ms: u32,
        _supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy {
        // MF's internal HTTP source does not expose whether the server
        // actually honoured a timeOffset parameter, so we cannot rely on
        // server-side seeking.  Handle the full position locally instead:
        // native mf_seek for formats that support it (MP3/AAC) and
        // read-and-discard as a fallback (FLAC).
        PlaybackLoadStrategy::exact_local(requested_position_ms)
    }

    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
        self.send_unit(|reply| WorkerJob::Load { request, reply })
    }

    fn seek(&mut self, _position_ms: u32) -> Result<PlaybackSeekAction, String> {
        // Native seek is not yet implemented; ask the controller to reload.
        Ok(PlaybackSeekAction::ReloadRequired)
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        self.send_u32(|reply| WorkerJob::CurrentPosition { reply })
    }

    fn set_volume(&mut self, _volume: f32) -> Result<(), String> {
        Ok(())
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
    #[allow(dead_code)]
    paused: bool,
}

// ---------------------------------------------------------------------------
// Worker thread
// ---------------------------------------------------------------------------

fn worker_main(rx: std::sync::mpsc::Receiver<WorkerJob>, event_hub: MfPlaybackEventHub) {
    // Initialise COM (MTA) and Media Foundation on this thread.
    let co_hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if co_hr.is_err() {
        log::error!("mf-worker: CoInitializeEx failed: {}", co_hr.message());
        return;
    }
    if let Err(e) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) } {
        log::error!("mf-worker: MFStartup failed: {e}");
        unsafe { CoUninitialize() };
        return;
    }

    let mut session: Option<ActiveSession> = None;
    let mut base_position_ms: Option<u32> = None;

    while let Ok(job) = rx.recv() {
        match job {
            WorkerJob::Load { request, reply } => {
                tear_down(&mut session);
                // Reset the EOS flag so a stale signal from the previous
                // session is not picked up by the controller.
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
    unsafe {
        if let Err(e) = MFShutdown() {
            log::warn!("mf-worker: MFShutdown failed: {e}");
        }
        CoUninitialize();
    }
}

fn tear_down(session: &mut Option<ActiveSession>) {
    if let Some(s) = session.take() {
        // Signal pump to stop, pause output, then join.
        s.ring.cancel.store(true, Ordering::Release);
        let _ = s.cpal_stream.pause();
        drop(s.cpal_stream);
        let _ = s.pump_handle.join();
    }
}

// ---------------------------------------------------------------------------
// Load – spawn pump thread, wait for format, build cpal stream
// ---------------------------------------------------------------------------

/// Data sent back by the pump thread once the source reader is ready.
struct PumpInit {
    channels: u16,
    sample_rate: u32,
    ring: Arc<AudioRing>,
}

fn do_load(
    request: &PlaybackBackendLoadRequest,
    event_hub: &MfPlaybackEventHub,
) -> Result<ActiveSession, String> {
    let url = request.request.url.to_string();
    let start_ms = request.local_start_position_ms;
    let autoplay = request.autoplay;

    log::info!(
        "mf-worker.load: url_len={} start_ms={start_ms} autoplay={autoplay}",
        url.len()
    );

    // The pump thread will create the MF source reader (COM object stays on
    // the thread that uses it) and report back the negotiated audio format
    // together with a shared ring buffer.
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<PumpInit, String>>();

    let pump_handle = std::thread::Builder::new()
        .name("mf-pump".into())
        .spawn(move || pump_entry(url, start_ms, init_tx))
        .map_err(|e| format!("Failed to spawn pump thread: {e}"))?;

    // Wait for the pump thread to finish initialisation.
    let init = init_rx
        .recv()
        .map_err(|_| "Pump thread exited before reporting init result.".to_string())??;

    log::info!(
        "mf-worker.load: negotiated format ch={} sr={}",
        init.channels,
        init.sample_rate
    );

    let ring = init.ring;

    // Build cpal output stream; on failure cancel the pump thread.
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
            let _ = pump_handle.join();
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
        paused: !autoplay,
    })
}

// ---------------------------------------------------------------------------
// Pump thread – owns the IMFSourceReader, decodes PCM into ring buffer
// ---------------------------------------------------------------------------

fn pump_entry(
    url: String,
    start_ms: u32,
    init_tx: std::sync::mpsc::Sender<Result<PumpInit, String>>,
) {
    // Each thread that touches COM needs its own initialisation.
    let co_hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if co_hr.is_err() {
        let _ = init_tx.send(Err(format!(
            "Pump thread COM init failed: {}",
            co_hr.message()
        )));
        return;
    }

    let result = pump_init_and_run(&url, start_ms, init_tx);
    if let Err(e) = &result {
        log::error!("mf-pump: terminated with error: {e}");
    }

    unsafe {
        CoUninitialize();
    }
}

fn pump_init_and_run(
    url: &str,
    start_ms: u32,
    init_tx: std::sync::mpsc::Sender<Result<PumpInit, String>>,
) -> Result<(), String> {
    // 1. Create the source reader (MF handles HTTP streaming internally).
    let (reader, channels, sample_rate) = mf_create_source_reader(url).map_err(|e| {
        let msg = format!("Failed to create MF source reader: {e}");
        let _ = init_tx.send(Err(msg.clone()));
        msg
    })?;

    // 2. Seek if the controller requested a non-zero start position.
    if start_ms > 0 {
        mf_seek(&reader, start_ms).map_err(|e| {
            let msg = format!("Failed to seek MF source reader to {start_ms} ms: {e}");
            let _ = init_tx.send(Err(msg.clone()));
            msg
        })?;
    }

    // 3. Create shared ring buffer and report back to the worker.
    let ring = Arc::new(AudioRing::new(channels, sample_rate));
    let _ = init_tx.send(Ok(PumpInit {
        channels,
        sample_rate,
        ring: Arc::clone(&ring),
    }));

    // 4. If a non-zero start was requested, verify the seek actually worked
    //    by checking the timestamp of the first decoded sample.  When MF's
    //    SetCurrentPosition silently fails (known FLAC bug), fall back to
    //    reading and discarding samples until the target position.
    if start_ms > 0 {
        pump_skip_to_position(&reader, &ring, start_ms)?;
    }

    // 5. Enter the read loop.
    pump_read_loop(&reader, &ring)
}

/// Verify that a preceding `mf_seek()` actually landed near the target
/// position.  If the first decoded sample's timestamp is far from the
/// target (e.g. MF's FLAC source ignoring SetCurrentPosition), fall back
/// to reading and discarding samples until we reach the desired position.
fn pump_skip_to_position(
    reader: &IMFSourceReader,
    ring: &AudioRing,
    target_ms: u32,
) -> Result<(), String> {
    let target_hns = i64::from(target_ms) * 10_000;
    let tolerance_hns: i64 = 500 * 10_000; // 500 ms

    let mut skipped_samples = 0u64;

    loop {
        if ring.cancel.load(Ordering::Acquire) {
            return Ok(());
        }

        let mut stream_flags = 0u32;
        let mut timestamp_hns: i64 = 0;
        let mut sample: Option<IMFSample> = None;

        unsafe {
            reader
                .ReadSample(
                    FIRST_AUDIO_STREAM,
                    0,
                    None,
                    Some(&mut stream_flags),
                    Some(&mut timestamp_hns),
                    Some(&mut sample),
                )
                .map_err(|e| format!("ReadSample failed during seek verification: {e}"))?;
        }

        if stream_flags & READERF_ERROR != 0 {
            ring.finished.store(true, Ordering::Release);
            return Err("MF source reader reported an error during seek verification.".to_string());
        }

        if stream_flags & READERF_END_OF_STREAM != 0 {
            ring.finished.store(true, Ordering::Release);
            log::warn!(
                "mf-pump: reached end of stream while seeking to {target_ms} ms (skipped {skipped_samples} samples)"
            );
            return Ok(());
        }

        // Determine whether this sample is close enough to the target.
        //
        // For the FIRST sample we use an absolute-distance check: if the
        // seek actually worked (MP3/AAC) the timestamp will be near the
        // target; if it silently failed (FLAC bug) the timestamp will be
        // ≈ 0 which is far from any meaningful target.
        //
        // When the target itself is very small (<= tolerance), the
        // distance check cannot distinguish success from failure, so we
        // fall through to the simple `>=` path – read-and-discard of a
        // sub-second amount is negligibly fast anyway.
        //
        // For SUBSEQUENT samples (already in discard mode) we simply wait
        // until the monotonically increasing timestamp reaches the target.
        let reached = if skipped_samples == 0 && target_hns > tolerance_hns {
            (timestamp_hns - target_hns).abs() <= tolerance_hns
        } else {
            timestamp_hns >= target_hns
        };

        if reached {
            // Push this sample into the ring buffer.
            if let Some(ref s) = sample {
                let pcm = extract_f32_from_sample(s)?;
                if !pcm.is_empty() {
                    ring.buf.lock().unwrap().extend(pcm.iter());
                }
            }
            if skipped_samples == 0 {
                log::info!(
                    "mf-pump: native seek verified at {} ms (target {target_ms} ms)",
                    timestamp_hns / 10_000
                );
            } else {
                log::info!(
                    "mf-pump: read-and-discard reached {} ms (target {target_ms} ms, skipped {skipped_samples} samples)",
                    timestamp_hns / 10_000
                );
            }
            return Ok(());
        }

        // Discard this sample and continue.
        skipped_samples += 1;
    }
}

fn pump_read_loop(reader: &IMFSourceReader, ring: &AudioRing) -> Result<(), String> {
    let two_seconds_of_samples = ring.sample_rate as usize * ring.channels as usize * 2;

    loop {
        if ring.cancel.load(Ordering::Acquire) {
            return Ok(());
        }

        // Back-pressure: sleep briefly when the ring already has ≥2 s of data.
        {
            let buf = ring.buf.lock().unwrap();
            if buf.len() >= two_seconds_of_samples {
                drop(buf);
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            }
        }

        // Read a decoded sample from MF.
        let mut stream_flags = 0u32;
        let mut sample: Option<IMFSample> = None;

        unsafe {
            reader
                .ReadSample(
                    FIRST_AUDIO_STREAM,
                    0, // no control flags
                    None,
                    Some(&mut stream_flags),
                    None,
                    Some(&mut sample),
                )
                .map_err(|e| format!("ReadSample failed: {e}"))?;
        }

        // Check for error flag.
        if stream_flags & READERF_ERROR != 0 {
            ring.finished.store(true, Ordering::Release);
            log::error!("mf-pump: source reader reported an error (flags=0x{stream_flags:X})");
            return Err("MF source reader reported an error.".to_string());
        }

        // Check for end-of-stream.
        if stream_flags & READERF_END_OF_STREAM != 0 {
            ring.finished.store(true, Ordering::Release);
            log::info!("mf-pump: end of stream");
            return Ok(());
        }

        // Extract f32 PCM and push into the ring buffer.
        if let Some(ref s) = sample {
            let pcm = extract_f32_from_sample(s)?;
            if !pcm.is_empty() {
                let mut buf = ring.buf.lock().unwrap();
                buf.extend(pcm.iter());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Media Foundation helpers
// ---------------------------------------------------------------------------

/// Create an `IMFSourceReader` from an HTTP(S) URL, configure it to output
/// decoded f32 PCM, and return the negotiated channel count & sample rate.
fn mf_create_source_reader(url: &str) -> Result<(IMFSourceReader, u16, u32), windows::core::Error> {
    unsafe {
        let wide_url = HSTRING::from(url);
        let reader: IMFSourceReader = MFCreateSourceReaderFromURL(&wide_url, None)?;

        // Tell MF we want decoded IEEE-float PCM.
        let desired: IMFMediaType = MFCreateMediaType()?;
        desired.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        desired.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_Float)?;

        reader.SetCurrentMediaType(FIRST_AUDIO_STREAM, None, &desired)?;

        // Read back the negotiated format.
        let actual: IMFMediaType = reader.GetCurrentMediaType(FIRST_AUDIO_STREAM)?;
        let channels = actual.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS)? as u16;
        let sample_rate = actual.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)?;

        Ok((reader, channels, sample_rate))
    }
}

/// Seek the source reader to the given position (milliseconds).
fn mf_seek(reader: &IMFSourceReader, position_ms: u32) -> Result<(), windows::core::Error> {
    unsafe {
        // MF positions are in 100-nanosecond units.
        let hns = i64::from(position_ms) * 10_000;
        let propvar = make_i64_propvariant(hns);
        reader.SetCurrentPosition(&GUID::zeroed(), &propvar)
    }
}

/// Construct a `PROPVARIANT` holding a VT_I8 (i64) value.
///
/// The windows crate 0.61 moved `PROPVARIANT` out of `windows::core`.
/// We use `VARIANT` (which is layout-compatible for our purpose) and
/// transmute, or build manually via the `Win32_System_Variant` types.
fn make_i64_propvariant(value: i64) -> windows::Win32::System::Com::StructuredStorage::PROPVARIANT {
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    // SAFETY: PROPVARIANT is a C union; we zero-init then set vt + value.
    unsafe {
        let mut pv: PROPVARIANT = std::mem::zeroed();
        // The in-memory layout: u16 vt, 3×u16 reserved, then the union payload.
        let ptr = &mut pv as *mut PROPVARIANT as *mut u8;
        // vt field at offset 0 (u16) – VT_I8 = 20
        *(ptr as *mut u16) = VT_I8.0 as u16;
        // i64 payload at offset 8
        *(ptr.add(8) as *mut i64) = value;
        pv
    }
}

/// Extract all f32 samples from an `IMFSample`.
fn extract_f32_from_sample(sample: &IMFSample) -> Result<Vec<f32>, String> {
    unsafe {
        let media_buf: IMFMediaBuffer = sample
            .ConvertToContiguousBuffer()
            .map_err(|e| format!("ConvertToContiguousBuffer failed: {e}"))?;

        let mut raw_ptr: *mut u8 = std::ptr::null_mut();
        let mut _max_len: u32 = 0;
        let mut current_len: u32 = 0;

        media_buf
            .Lock(&mut raw_ptr, Some(&mut _max_len), Some(&mut current_len))
            .map_err(|e| format!("IMFMediaBuffer::Lock failed: {e}"))?;

        let f32_count = current_len as usize / std::mem::size_of::<f32>();
        let slice = std::slice::from_raw_parts(raw_ptr as *const f32, f32_count);
        let data = slice.to_vec();

        media_buf
            .Unlock()
            .map_err(|e| format!("IMFMediaBuffer::Unlock failed: {e}"))?;

        Ok(data)
    }
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
            config,
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
                    log::info!("mf-cpal: end-of-stream signalled to event hub");
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
