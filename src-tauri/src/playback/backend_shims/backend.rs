use opensubsonic_client::PreparedBinaryRequest;

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
    fn plan_load(
        &self,
        requested_position_ms: u32,
        supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy;
    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String>;
    fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String>;
    fn current_position_ms(&self) -> Result<u32, String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}
