use opensubsonic_client::PreparedBinaryRequest;

use crate::playback::backend_shims::backend::PlaybackBackend;

#[derive(Debug, Default)]
struct AndroidPlaybackBackend;

impl PlaybackBackend for AndroidPlaybackBackend {
    fn load(
        &mut self,
        _request: PreparedBinaryRequest,
        _start_position_ms: u32,
        _autoplay: bool,
    ) -> Result<(), String> {
        Err("Playback backend is only implemented for Windows.".to_string())
    }

    fn seek(&mut self, _position_ms: u32) -> Result<(), String> {
        Err("Playback backend is only implemented for Windows.".to_string())
    }

    fn pause(&mut self) -> Result<(), String> {
        Err("Playback backend is only implemented for Windows.".to_string())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

pub fn create_playback_backend() -> Box<dyn PlaybackBackend> {
    Box::new(AndroidPlaybackBackend)
}
