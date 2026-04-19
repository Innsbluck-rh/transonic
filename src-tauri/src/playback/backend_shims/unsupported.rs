use crate::playback::backend_shims::backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};
use crate::models::PlaybackCapabilities;

#[derive(Debug, Default)]
struct UnsupportedPlaybackBackend;

impl PlaybackBackend for UnsupportedPlaybackBackend {
    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities {
            gapless_playback: false,
        }
    }

    fn plan_load(
        &self,
        requested_position_ms: u32,
        supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy {
        PlaybackLoadStrategy::split_by_stream_offset(requested_position_ms, supports_stream_offset)
    }

    fn load(&mut self, _request: PlaybackBackendLoadRequest) -> Result<(), String> {
        Err("Playback backend is only implemented for Windows and Android.".to_string())
    }

    fn seek(&mut self, _position_ms: u32) -> Result<PlaybackSeekAction, String> {
        Err("Playback backend is only implemented for Windows and Android.".to_string())
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        Err("Playback backend is only implemented for Windows and Android.".to_string())
    }

    fn pause(&mut self) -> Result<(), String> {
        Err("Playback backend is only implemented for Windows and Android.".to_string())
    }

    fn resume(&mut self) -> Result<(), String> {
        Err("Playback backend is only implemented for Windows and Android.".to_string())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

pub fn create_playback_backend() -> Box<dyn PlaybackBackend> {
    Box::new(UnsupportedPlaybackBackend)
}
