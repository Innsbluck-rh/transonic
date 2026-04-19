use crate::playback::android_mobile_plugin::{
    AndroidPlaybackBridge, AndroidPlaybackHeader, AndroidPreparedMediaRequest,
};
use crate::playback::backend_shims::backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};
use crate::models::PlaybackCapabilities;

#[derive(Debug, Clone)]
struct AndroidPlaybackBackend {
    bridge: AndroidPlaybackBridge,
}

impl PlaybackBackend for AndroidPlaybackBackend {
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

    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
        let headers = request
            .request
            .headers
            .iter()
            .map(|(name, value)| {
                Ok(AndroidPlaybackHeader {
                    name: name.as_str().to_string(),
                    value: value
                        .to_str()
                        .map_err(|error| {
                            format!("Invalid stream header value for {name}: {error}")
                        })?
                        .to_string(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        self.bridge
            .load_prepared_media(&AndroidPreparedMediaRequest {
                media_id: request.media_id,
                stream_url: request.request.url.to_string(),
                headers,
                absolute_start_position_ms: request.absolute_start_position_ms,
                local_start_position_ms: request.local_start_position_ms,
                autoplay: request.autoplay,
                title: request.title,
                artist: request.artist,
                album: request.album,
                artwork_path: request.artwork_path,
            })
    }

    fn seek(&mut self, position_ms: u32) -> Result<PlaybackSeekAction, String> {
        self.bridge.seek_to(position_ms)?;
        Ok(PlaybackSeekAction::Applied)
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        self.bridge.current_position_ms()
    }

    fn pause(&mut self) -> Result<(), String> {
        self.bridge.pause()
    }

    fn resume(&mut self) -> Result<(), String> {
        self.bridge.play()
    }

    fn stop(&mut self) -> Result<(), String> {
        self.bridge.stop()
    }
}

pub fn create_playback_backend(bridge: AndroidPlaybackBridge) -> Box<dyn PlaybackBackend> {
    Box::new(AndroidPlaybackBackend { bridge })
}
