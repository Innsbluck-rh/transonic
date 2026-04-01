// use opensubsonic_client::PreparedBinaryRequest;

// pub trait PlaybackBackend: Send {
//     fn load(
//         &mut self,
//         request: PreparedBinaryRequest,
//         start_position_ms: u32,
//         autoplay: bool,
//     ) -> Result<(), String>;
//     fn seek(&mut self, position_ms: u32) -> Result<(), String>;
//     fn pause(&mut self) -> Result<(), String>;
//     fn stop(&mut self) -> Result<(), String>;
// }

// pub fn create_playback_backend() -> Box<dyn PlaybackBackend> {
//     #[cfg(target_os = "windows")]
//     {
//         Box::new(WindowsPlaybackBackend::default())
//     }

//     #[cfg(not(target_os = "windows"))]
//     {
//         Box::new(UnsupportedPlaybackBackend)
//     }
// }

// #[cfg(not(target_os = "windows"))]
// #[derive(Debug, Default)]
// struct UnsupportedPlaybackBackend;

// #[cfg(not(target_os = "windows"))]
// impl PlaybackBackend for UnsupportedPlaybackBackend {
//     fn load(
//         &mut self,
//         _request: PreparedBinaryRequest,
//         _start_position_ms: u32,
//         _autoplay: bool,
//     ) -> Result<(), String> {
//         Err("Playback backend is only implemented for Windows.".to_string())
//     }

//     fn seek(&mut self, _position_ms: u32) -> Result<(), String> {
//         Err("Playback backend is only implemented for Windows.".to_string())
//     }

//     fn pause(&mut self) -> Result<(), String> {
//         Err("Playback backend is only implemented for Windows.".to_string())
//     }

//     fn stop(&mut self) -> Result<(), String> {
//         Ok(())
//     }
// }

// #[cfg(target_os = "windows")]
// #[derive(Debug)]
// enum PlaybackJob {
//     Load {
//         request: PreparedBinaryRequest,
//         start_position_ms: u32,
//         autoplay: bool,
//         result_tx: std::sync::mpsc::Sender<Result<(), String>>,
//     },
//     Seek {
//         position_ms: u32,
//         result_tx: std::sync::mpsc::Sender<Result<(), String>>,
//     },
//     Pause {
//         result_tx: std::sync::mpsc::Sender<Result<(), String>>,
//     },
//     Stop {
//         result_tx: std::sync::mpsc::Sender<Result<(), String>>,
//     },
// }

// #[cfg(target_os = "windows")]
// #[derive(Clone)]
// struct WindowsPlaybackBackend {
//     tx: std::sync::mpsc::Sender<PlaybackJob>,
// }

// #[cfg(target_os = "windows")]
// impl Default for WindowsPlaybackBackend {
//     fn default() -> Self {
//         let (tx, rx) = std::sync::mpsc::channel::<PlaybackJob>();
//         std::thread::spawn(move || playback_worker_loop(rx));
//         Self { tx }
//     }
// }

// #[cfg(target_os = "windows")]
// impl WindowsPlaybackBackend {
//     fn request_result(
//         &self,
//         build_job: impl FnOnce(std::sync::mpsc::Sender<Result<(), String>>) -> PlaybackJob,
//     ) -> Result<(), String> {
//         let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<(), String>>();
//         self.tx
//             .send(build_job(result_tx))
//             .map_err(|_| "Playback worker is unavailable.".to_string())?;
//         result_rx
//             .recv()
//             .unwrap_or_else(|_| Err("Playback worker terminated unexpectedly.".to_string()))
//     }
// }

// #[cfg(target_os = "windows")]
// impl PlaybackBackend for WindowsPlaybackBackend {
//     fn load(
//         &mut self,
//         request: PreparedBinaryRequest,
//         start_position_ms: u32,
//         autoplay: bool,
//     ) -> Result<(), String> {
//         self.request_result(|result_tx| PlaybackJob::Load {
//             request,
//             start_position_ms,
//             autoplay,
//             result_tx,
//         })
//     }

//     fn seek(&mut self, position_ms: u32) -> Result<(), String> {
//         self.request_result(|result_tx| PlaybackJob::Seek {
//             position_ms,
//             result_tx,
//         })
//     }

//     fn pause(&mut self) -> Result<(), String> {
//         self.request_result(|result_tx| PlaybackJob::Pause { result_tx })
//     }

//     fn stop(&mut self) -> Result<(), String> {
//         self.request_result(|result_tx| PlaybackJob::Stop { result_tx })
//     }
// }

// #[cfg(target_os = "windows")]
// fn playback_worker_loop(rx: std::sync::mpsc::Receiver<PlaybackJob>) {
//     let mut worker = WindowsPlaybackWorker::default();

//     while let Ok(job) = rx.recv() {
//         match job {
//             PlaybackJob::Load {
//                 request,
//                 start_position_ms,
//                 autoplay,
//                 result_tx,
//             } => {
//                 let _ = result_tx.send(worker.load(request, start_position_ms, autoplay));
//             }
//             PlaybackJob::Seek {
//                 position_ms,
//                 result_tx,
//             } => {
//                 let _ = result_tx.send(worker.seek(position_ms));
//             }
//             PlaybackJob::Pause { result_tx } => {
//                 let _ = result_tx.send(worker.pause());
//             }
//             PlaybackJob::Stop { result_tx } => {
//                 let _ = result_tx.send(worker.stop());
//             }
//         }
//     }
// }

// #[cfg(target_os = "windows")]
// #[derive(Default)]
// struct WindowsPlaybackWorker {
//     output_stream: Option<rodio::OutputStream>,
//     output_stream_handle: Option<rodio::OutputStreamHandle>,
//     sink: Option<rodio::Sink>,
// }

// #[cfg(target_os = "windows")]
// impl WindowsPlaybackWorker {
//     fn ensure_output_stream(&mut self) -> Result<&rodio::OutputStreamHandle, String> {
//         if self.output_stream.is_none() || self.output_stream_handle.is_none() {
//             let (output_stream, output_stream_handle) = rodio::OutputStream::try_default()
//                 .map_err(|error| {
//                     format!("Failed to initialize the Windows audio output: {error}")
//                 })?;
//             self.output_stream = Some(output_stream);
//             self.output_stream_handle = Some(output_stream_handle);
//         }

//         self.output_stream_handle
//             .as_ref()
//             .ok_or_else(|| "Windows audio output is unavailable.".to_string())
//     }

//     fn load(
//         &mut self,
//         request: PreparedBinaryRequest,
//         start_position_ms: u32,
//         autoplay: bool,
//     ) -> Result<(), String> {
//         use rodio::Source;

//         let response = reqwest::blocking::Client::new()
//             .get(request.url)
//             .headers(request.headers)
//             .send()
//             .map_err(|error| format!("Failed to fetch the stream payload: {error}"))?;

//         let status = response.status();
//         if !status.is_success() {
//             return Err(format!(
//                 "The server returned HTTP {} for the stream request.",
//                 status.as_u16()
//             ));
//         }

//         let content_type = response
//             .headers()
//             .get(reqwest::header::CONTENT_TYPE)
//             .and_then(|value| value.to_str().ok())
//             .unwrap_or("application/octet-stream")
//             .to_ascii_lowercase();
//         if !is_supported_audio_content_type(&content_type) {
//             return Err(format!(
//                 "Unsupported stream content type returned by server: {content_type}"
//             ));
//         }

//         let bytes = response
//             .bytes()
//             .map_err(|error| format!("Failed to read stream bytes: {error}"))?;
//         let source = rodio::Decoder::new(std::io::BufReader::new(std::io::Cursor::new(
//             bytes.to_vec(),
//         )))
//         .map_err(|error| format!("Failed to decode stream bytes: {error}"))?
//         .skip_duration(std::time::Duration::from_millis(u64::from(
//             start_position_ms,
//         )));
//         let sink = rodio::Sink::try_new(self.ensure_output_stream()?)
//             .map_err(|error| format!("Failed to create the playback sink: {error}"))?;

//         if let Some(existing_sink) = self.sink.take() {
//             existing_sink.stop();
//         }

//         sink.append(source);
//         if autoplay {
//             sink.play();
//         } else {
//             sink.pause();
//         }
//         self.sink = Some(sink);

//         Ok(())
//     }

//     fn seek(&mut self, position_ms: u32) -> Result<(), String> {
//         let Some(sink) = self.sink.as_ref() else {
//             log::warn!("backend.seek: no sink is loaded");
//             return Err("No stream is loaded for playback.".to_string());
//         };
//         sink.try_seek(std::time::Duration::from_millis(u64::from(position_ms)))
//             .map_err(|error| format!("Failed to seek the playback sink: {error}"))
//     }

//     fn pause(&mut self) -> Result<(), String> {
//         let Some(sink) = self.sink.as_ref() else {
//             log::warn!("backend.pause: no sink is loaded");
//             return Err("No stream is loaded for playback.".to_string());
//         };
//         sink.pause();
//         Ok(())
//     }

//     fn stop(&mut self) -> Result<(), String> {
//         if let Some(sink) = self.sink.take() {
//             sink.stop();
//         }
//         Ok(())
//     }
// }

// #[cfg(target_os = "windows")]
// fn is_supported_audio_content_type(content_type: &str) -> bool {
//     content_type.starts_with("audio/")
//         || content_type.starts_with("application/octet-stream")
//         || content_type.starts_with("application/ogg")
// }
