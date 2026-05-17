use crate::models::PlaybackCapabilities;
use crate::playback::android_mobile_plugin::{
    AndroidActivatePreparedMediaRequest, AndroidPlaybackBridge, AndroidPlaybackHeader,
    AndroidPreparedMediaRequest, AndroidUpdateMediaArtworkRequest,
};
use crate::playback::backend_shims::backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};

trait AndroidPlaybackControl: Send {
    fn play(&self) -> Result<(), String>;
    fn load_prepared_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String>;
    fn prepare_next_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String>;
    fn activate_prepared_media(
        &self,
        payload: &AndroidActivatePreparedMediaRequest,
    ) -> Result<(), String>;
    fn clear_prepared_media(&self) -> Result<(), String>;
    fn update_media_artwork(
        &self,
        payload: &AndroidUpdateMediaArtworkRequest,
    ) -> Result<(), String>;
    fn pause(&self) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
    fn seek_to(&self, position_ms: u32) -> Result<(), String>;
    fn current_position_ms(&self) -> Result<u32, String>;
}

impl AndroidPlaybackControl for AndroidPlaybackBridge {
    fn play(&self) -> Result<(), String> {
        Self::play(self)
    }

    fn load_prepared_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String> {
        Self::load_prepared_media(self, payload)
    }

    fn prepare_next_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String> {
        Self::prepare_next_media(self, payload)
    }

    fn activate_prepared_media(
        &self,
        payload: &AndroidActivatePreparedMediaRequest,
    ) -> Result<(), String> {
        Self::activate_prepared_media(self, payload)
    }

    fn clear_prepared_media(&self) -> Result<(), String> {
        Self::clear_prepared_media(self)
    }

    fn update_media_artwork(
        &self,
        payload: &AndroidUpdateMediaArtworkRequest,
    ) -> Result<(), String> {
        Self::update_media_artwork(self, payload)
    }

    fn pause(&self) -> Result<(), String> {
        Self::pause(self)
    }

    fn stop(&self) -> Result<(), String> {
        Self::stop(self)
    }

    fn seek_to(&self, position_ms: u32) -> Result<(), String> {
        Self::seek_to(self, position_ms)
    }

    fn current_position_ms(&self) -> Result<u32, String> {
        Self::current_position_ms(self)
    }
}

struct AndroidPlaybackBackend {
    bridge: Box<dyn AndroidPlaybackControl>,
    next_prepare_generation: u64,
    prepared_generation: Option<u64>,
    next_media_instance_id: u64,
}

impl AndroidPlaybackBackend {
    fn new(bridge: Box<dyn AndroidPlaybackControl>) -> Self {
        Self {
            bridge,
            next_prepare_generation: 0,
            prepared_generation: None,
            next_media_instance_id: 0,
        }
    }

    fn to_request_payload(
        &mut self,
        request: PlaybackBackendLoadRequest,
    ) -> Result<AndroidPreparedMediaRequest, String> {
        self.next_media_instance_id = self.next_media_instance_id.wrapping_add(1);
        let media_instance_id = format!("{}#{}", request.media_id, self.next_media_instance_id);
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

        Ok(AndroidPreparedMediaRequest {
            media_id: media_instance_id,
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
}

impl PlaybackBackend for AndroidPlaybackBackend {
    fn capabilities(&self) -> PlaybackCapabilities {
        PlaybackCapabilities {
            gapless_playback: true,
        }
    }

    fn plan_load(
        &self,
        requested_position_ms: u32,
        _supports_stream_offset: bool,
    ) -> PlaybackLoadStrategy {
        // Keep Android takeover/reload starts local. Some servers expose the
        // transcodeOffset extension but still serve the stream from the
        // beginning, and ExoPlayer cannot tell us whether the URL-side offset
        // was honoured.
        PlaybackLoadStrategy::exact_local(requested_position_ms)
    }

    fn load(&mut self, request: PlaybackBackendLoadRequest) -> Result<(), String> {
        let payload = self.to_request_payload(request)?;
        self.prepared_generation = None;
        self.bridge.load_prepared_media(&payload)
    }

    fn prepare(&mut self, request: PlaybackBackendLoadRequest) -> Result<u64, String> {
        let payload = self.to_request_payload(request)?;
        self.bridge.prepare_next_media(&payload)?;
        self.next_prepare_generation = self.next_prepare_generation.wrapping_add(1);
        let generation = self.next_prepare_generation;
        self.prepared_generation = Some(generation);
        Ok(generation)
    }

    fn activate_prepared(&mut self, autoplay: bool) -> Result<(), String> {
        let Some(_generation) = self.prepared_generation.take() else {
            return Err("No prepared Android playback media is available.".to_string());
        };
        self.bridge
            .activate_prepared_media(&AndroidActivatePreparedMediaRequest { autoplay })
    }

    fn clear_prepared(&mut self) {
        self.prepared_generation = None;
        if let Err(error) = self.bridge.clear_prepared_media() {
            log::warn!("android backend: failed to clear prepared media: {error}");
        }
    }

    fn update_artwork(
        &mut self,
        media_id: &str,
        artwork_path: Option<String>,
    ) -> Result<(), String> {
        let Some(artwork_path) = artwork_path else {
            return Ok(());
        };
        self.bridge
            .update_media_artwork(&AndroidUpdateMediaArtworkRequest {
                media_id: media_id.to_string(),
                artwork_path,
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
        self.prepared_generation = None;
        self.bridge.stop()
    }
}

pub fn create_playback_backend(bridge: AndroidPlaybackBridge) -> Box<dyn PlaybackBackend> {
    Box::new(AndroidPlaybackBackend::new(Box::new(bridge)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use opensubsonic_client::PreparedBinaryRequest;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::Url;

    #[derive(Default)]
    struct MockAndroidPlaybackControl {
        loaded_media_ids: Vec<String>,
        prepared_media_ids: Vec<String>,
        activated_autoplay: Vec<bool>,
        updated_artwork: Vec<AndroidUpdateMediaArtworkRequest>,
        clear_prepared_calls: usize,
        stop_calls: usize,
    }

    impl AndroidPlaybackControl for std::sync::Arc<std::sync::Mutex<MockAndroidPlaybackControl>> {
        fn play(&self) -> Result<(), String> {
            Ok(())
        }

        fn load_prepared_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String> {
            self.lock()
                .unwrap()
                .loaded_media_ids
                .push(payload.media_id.clone());
            Ok(())
        }

        fn prepare_next_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String> {
            self.lock()
                .unwrap()
                .prepared_media_ids
                .push(payload.media_id.clone());
            Ok(())
        }

        fn activate_prepared_media(
            &self,
            payload: &AndroidActivatePreparedMediaRequest,
        ) -> Result<(), String> {
            self.lock()
                .unwrap()
                .activated_autoplay
                .push(payload.autoplay);
            Ok(())
        }

        fn clear_prepared_media(&self) -> Result<(), String> {
            self.lock().unwrap().clear_prepared_calls += 1;
            Ok(())
        }

        fn update_media_artwork(
            &self,
            payload: &AndroidUpdateMediaArtworkRequest,
        ) -> Result<(), String> {
            self.lock().unwrap().updated_artwork.push(payload.clone());
            Ok(())
        }

        fn pause(&self) -> Result<(), String> {
            Ok(())
        }

        fn stop(&self) -> Result<(), String> {
            self.lock().unwrap().stop_calls += 1;
            Ok(())
        }

        fn seek_to(&self, _position_ms: u32) -> Result<(), String> {
            Ok(())
        }

        fn current_position_ms(&self) -> Result<u32, String> {
            Ok(0)
        }
    }

    fn load_request(media_id: &str) -> PlaybackBackendLoadRequest {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("Bearer test"));
        PlaybackBackendLoadRequest {
            request: PreparedBinaryRequest {
                url: Url::parse("https://example.com/rest/stream.view?id=test").unwrap(),
                headers,
            },
            media_id: media_id.to_string(),
            title: "Song".to_string(),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            artwork_path: None,
            absolute_start_position_ms: 0,
            local_start_position_ms: 0,
            autoplay: true,
        }
    }

    #[test]
    fn prepare_increments_generation_and_activate_clears_local_state() {
        let bridge =
            std::sync::Arc::new(std::sync::Mutex::new(MockAndroidPlaybackControl::default()));
        let mut backend = AndroidPlaybackBackend::new(Box::new(bridge.clone()));

        let first_generation = backend.prepare(load_request("song-a")).unwrap();
        let second_generation = backend.prepare(load_request("song-b")).unwrap();

        assert_eq!(first_generation, 1);
        assert_eq!(second_generation, 2);
        backend.activate_prepared(true).unwrap();
        assert_eq!(
            backend.activate_prepared(true).unwrap_err(),
            "No prepared Android playback media is available."
        );

        let state = bridge.lock().unwrap();
        assert_eq!(state.prepared_media_ids.len(), 2);
        assert_eq!(state.activated_autoplay, vec![true]);
    }

    #[test]
    fn clear_prepared_and_stop_reset_prepared_generation() {
        let bridge =
            std::sync::Arc::new(std::sync::Mutex::new(MockAndroidPlaybackControl::default()));
        let mut backend = AndroidPlaybackBackend::new(Box::new(bridge.clone()));

        backend.prepare(load_request("song-a")).unwrap();
        backend.clear_prepared();
        assert_eq!(
            backend.activate_prepared(false).unwrap_err(),
            "No prepared Android playback media is available."
        );

        backend.prepare(load_request("song-b")).unwrap();
        backend.stop().unwrap();
        assert_eq!(
            backend.activate_prepared(false).unwrap_err(),
            "No prepared Android playback media is available."
        );

        let state = bridge.lock().unwrap();
        assert_eq!(state.clear_prepared_calls, 1);
        assert_eq!(state.stop_calls, 1);
    }

    #[test]
    fn load_uses_unique_media_instance_ids() {
        let bridge =
            std::sync::Arc::new(std::sync::Mutex::new(MockAndroidPlaybackControl::default()));
        let mut backend = AndroidPlaybackBackend::new(Box::new(bridge.clone()));

        backend.load(load_request("song-a")).unwrap();
        backend.load(load_request("song-a")).unwrap();

        let state = bridge.lock().unwrap();
        assert_eq!(state.loaded_media_ids.len(), 2);
        assert_ne!(state.loaded_media_ids[0], state.loaded_media_ids[1]);
    }

    #[test]
    fn update_artwork_forwards_song_id_and_path() {
        let bridge =
            std::sync::Arc::new(std::sync::Mutex::new(MockAndroidPlaybackControl::default()));
        let mut backend = AndroidPlaybackBackend::new(Box::new(bridge.clone()));

        backend
            .update_artwork("song-a", Some("cache/cover.png".to_string()))
            .unwrap();

        let state = bridge.lock().unwrap();
        assert_eq!(state.updated_artwork.len(), 1);
        assert_eq!(state.updated_artwork[0].media_id, "song-a");
        assert_eq!(state.updated_artwork[0].artwork_path, "cache/cover.png");
    }

    #[test]
    fn plan_load_uses_exact_local_start_position() {
        let bridge =
            std::sync::Arc::new(std::sync::Mutex::new(MockAndroidPlaybackControl::default()));
        let backend = AndroidPlaybackBackend::new(Box::new(bridge));

        assert_eq!(
            backend.plan_load(7_500, true),
            PlaybackLoadStrategy {
                stream_offset_seconds: None,
                local_start_position_ms: 7_500,
            }
        );
    }
}
