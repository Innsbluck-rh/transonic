use opensubsonic_client::PreparedBinaryRequest;

use crate::models::{
    PlaybackActualStreamInfo, PlaybackCapabilities, PlaybackStreamMode, PlaybackTranscodingCodec,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PlaybackBackendLoadRequest {
    pub request: PreparedBinaryRequest,
    pub media_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_path: Option<String>,
    pub absolute_start_position_ms: u32,
    pub local_start_position_ms: u32,
    pub autoplay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackLoadStrategy {
    pub stream_offset_seconds: Option<u32>,
    pub local_start_position_ms: u32,
}

impl PlaybackLoadStrategy {
    pub fn exact_local(position_ms: u32) -> Self {
        Self {
            stream_offset_seconds: None,
            local_start_position_ms: position_ms,
        }
    }

    #[allow(dead_code)]
    pub fn split_by_stream_offset(position_ms: u32, supports_stream_offset: bool) -> Self {
        if !supports_stream_offset || position_ms == 0 {
            return Self::exact_local(position_ms);
        }

        Self {
            stream_offset_seconds: Some(position_ms / 1000),
            local_start_position_ms: position_ms % 1000,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackSeekAction {
    Applied,
    ReloadRequired,
}

pub trait PlaybackBackend: Send {
    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities {
            gapless_playback: false,
            transcoding_codecs: vec![PlaybackTranscodingCodec::Mp3],
        }
    }

    fn should_estimate_stream_content_length(
        &self,
        _stream_mode: PlaybackStreamMode,
        _raw_stream: bool,
    ) -> bool {
        true
    }

    fn plan_load(
        &self,
        requested_position_ms: u32,
        supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy;
    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String>;
    fn prepare(&mut self, _request: PlaybackBackendLoadRequest) -> Result<u64, String> {
        Err("Gapless playback is unavailable on this backend.".to_string())
    }
    fn activate_prepared(&mut self, _autoplay: bool) -> Result<(), String> {
        Err("Gapless playback is unavailable on this backend.".to_string())
    }
    fn clear_prepared(&mut self) {}
    fn update_artwork(
        &mut self,
        _media_id: &str,
        _artwork_path: Option<String>,
    ) -> Result<(), String> {
        Ok(())
    }
    fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String>;
    fn current_position_ms(&self) -> Result<u32, String>;
    fn current_stream_info(&self) -> Result<Option<PlaybackActualStreamInfo>, String> {
        Ok(None)
    }
    fn set_volume(&mut self, volume: f32) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn resume(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}
