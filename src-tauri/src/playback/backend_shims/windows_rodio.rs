use crate::playback::backend_shims::backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};

pub fn create_playback_backend() -> Box<dyn PlaybackBackend> {
    Box::new(WindowsPlaybackBackend::default())
}

#[derive(Debug)]
enum PlaybackJob {
    Load {
        request: PlaybackBackendLoadRequest,
        result_tx: std::sync::mpsc::Sender<Result<(), String>>,
    },
    CurrentPosition {
        result_tx: std::sync::mpsc::Sender<Result<u32, String>>,
    },
    Pause {
        result_tx: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Resume {
        result_tx: std::sync::mpsc::Sender<Result<(), String>>,
    },
    Stop {
        result_tx: std::sync::mpsc::Sender<Result<(), String>>,
    },
}

#[derive(Clone)]
struct WindowsPlaybackBackend {
    tx: std::sync::mpsc::Sender<PlaybackJob>,
}

impl Default for WindowsPlaybackBackend {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<PlaybackJob>();
        std::thread::spawn(move || playback_worker_loop(rx));
        Self { tx }
    }
}

impl WindowsPlaybackBackend {
    fn request_result(
        &self,
        build_job: impl FnOnce(std::sync::mpsc::Sender<Result<(), String>>) -> PlaybackJob,
    ) -> Result<(), String> {
        let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        self.tx
            .send(build_job(result_tx))
            .map_err(|_| "Playback worker is unavailable.".to_string())?;
        result_rx
            .recv()
            .unwrap_or_else(|_| Err("Playback worker terminated unexpectedly.".to_string()))
    }

    fn request_position_result(
        &self,
        build_job: impl FnOnce(std::sync::mpsc::Sender<Result<u32, String>>) -> PlaybackJob,
    ) -> Result<u32, String> {
        let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<u32, String>>();
        self.tx
            .send(build_job(result_tx))
            .map_err(|_| "Playback worker is unavailable.".to_string())?;
        result_rx
            .recv()
            .unwrap_or_else(|_| Err("Playback worker terminated unexpectedly.".to_string()))
    }
}

impl PlaybackBackend for WindowsPlaybackBackend {
    fn plan_load(
        &self,
        requested_position_ms: u32,
        _supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy {
        PlaybackLoadStrategy::exact_local(requested_position_ms)
    }

    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
        self.request_result(|result_tx| PlaybackJob::Load { request, result_tx })
    }

    fn seek(&mut self, _position_ms: u32) -> Result<PlaybackSeekAction, String> {
        Ok(PlaybackSeekAction::ReloadRequired)
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        self.request_position_result(|result_tx| PlaybackJob::CurrentPosition { result_tx })
    }

    fn pause(&mut self) -> Result<(), String> {
        self.request_result(|result_tx| PlaybackJob::Pause { result_tx })
    }

    fn resume(&mut self) -> Result<(), String> {
        self.request_result(|result_tx| PlaybackJob::Resume { result_tx })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.request_result(|result_tx| PlaybackJob::Stop { result_tx })
    }
}

fn playback_worker_loop(rx: std::sync::mpsc::Receiver<PlaybackJob>) {
    let mut worker = WindowsPlaybackWorker::default();

    while let Ok(job) = rx.recv() {
        match job {
            PlaybackJob::Load { request, result_tx } => {
                let _ = result_tx.send(worker.load(request));
            }
            PlaybackJob::CurrentPosition { result_tx } => {
                let _ = result_tx.send(worker.current_position_ms());
            }
            PlaybackJob::Pause { result_tx } => {
                let _ = result_tx.send(worker.pause());
            }
            PlaybackJob::Resume { result_tx } => {
                let _ = result_tx.send(worker.resume());
            }
            PlaybackJob::Stop { result_tx } => {
                let _ = result_tx.send(worker.stop());
            }
        }
    }
}

#[derive(Default)]
struct WindowsPlaybackWorker {
    output_stream: Option<rodio::MixerDeviceSink>,
    sink: Option<rodio::Player>,
    loaded_absolute_position_ms: Option<u32>,
}

impl WindowsPlaybackWorker {
    fn ensure_output_stream(&mut self) -> Result<&rodio::MixerDeviceSink, String> {
        if self.output_stream.is_none() {
            let output_stream = rodio::DeviceSinkBuilder::open_default_sink().map_err(|error| {
                format!("Failed to initialize the Windows audio output: {error}")
            })?;
            self.output_stream = Some(output_stream);
        }

        self.output_stream
            .as_ref()
            .ok_or_else(|| "Windows audio output is unavailable.".to_string())
    }

    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
        use rodio::{Decoder, Source};

        let response = reqwest::blocking::Client::new()
            .get(request.request.url)
            .headers(request.request.headers)
            .send()
            .map_err(|error| format!("Failed to fetch the stream payload: {error}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "The server returned HTTP {} for the stream request.",
                status.as_u16()
            ));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_ascii_lowercase();
        if !is_supported_audio_content_type(&content_type) {
            return Err(format!(
                "Unsupported stream content type returned by server: {content_type}"
            ));
        }

        let bytes = response
            .bytes()
            .map_err(|error| format!("Failed to read stream bytes: {error}"))?;
        let byte_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let mut decoder = Decoder::builder()
            .with_data(std::io::Cursor::new(bytes.to_vec()))
            .with_byte_len(byte_len)
            .with_seekable(true)
            .with_mime_type(normalize_mime_type(&content_type))
            .build()
            .map_err(|error| format!("Failed to decode stream bytes: {error}"))?;
        if request.local_start_position_ms > 0 {
            decoder
                .try_seek(std::time::Duration::from_millis(u64::from(
                    request.local_start_position_ms,
                )))
                .map_err(|error| format!("Failed to seek decoded stream bytes: {error}"))?;
        }
        let sink = rodio::Player::connect_new(self.ensure_output_stream()?.mixer());

        if let Some(existing_sink) = self.sink.take() {
            existing_sink.stop();
        }

        sink.append(decoder);
        if request.autoplay {
            sink.play();
        } else {
            sink.pause();
        }
        self.sink = Some(sink);
        self.loaded_absolute_position_ms = Some(request.absolute_start_position_ms);

        Ok(())
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        let Some(sink) = self.sink.as_ref() else {
            log::warn!("backend.current_position_ms: no sink is loaded");
            return Err("No stream is loaded for playback.".to_string());
        };

        let base_position_ms = self
            .loaded_absolute_position_ms
            .ok_or_else(|| "Playback position base is unavailable.".to_string())?;
        let elapsed_ms = sink.get_pos().as_millis();
        let elapsed_ms = u32::try_from(elapsed_ms).unwrap_or(u32::MAX);
        Ok(base_position_ms.saturating_add(elapsed_ms))
    }

    fn pause(&mut self) -> Result<(), String> {
        let Some(sink) = self.sink.as_ref() else {
            log::warn!("backend.pause: no sink is loaded");
            return Err("No stream is loaded for playback.".to_string());
        };
        sink.pause();
        Ok(())
    }

    fn resume(&mut self) -> Result<(), String> {
        let Some(sink) = self.sink.as_ref() else {
            log::warn!("backend.resume: no sink is loaded");
            return Err("No stream is loaded for playback.".to_string());
        };
        sink.play();
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.loaded_absolute_position_ms = None;
        Ok(())
    }
}

fn is_supported_audio_content_type(content_type: &str) -> bool {
    content_type.starts_with("audio/")
        || content_type.starts_with("application/octet-stream")
        || content_type.starts_with("application/ogg")
}

fn normalize_mime_type(content_type: &str) -> &str {
    content_type
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("application/octet-stream")
}
