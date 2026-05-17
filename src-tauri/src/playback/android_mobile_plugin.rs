use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder as PluginBuilder, PluginHandle, TauriPlugin},
    Manager, Wry,
};

use super::native_events::AndroidPlaybackEventHub;

const PLUGIN_IDENTIFIER: &str = "com.innsb.transonic.playback";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidPlaybackHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidPreparedMediaRequest {
    pub media_id: String,
    pub stream_url: String,
    pub headers: Vec<AndroidPlaybackHeader>,
    pub absolute_start_position_ms: u32,
    pub local_start_position_ms: u32,
    pub autoplay: bool,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidActivatePreparedMediaRequest {
    pub autoplay: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidUpdateMediaArtworkRequest {
    pub media_id: String,
    pub artwork_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidPositionResponse {
    position_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidDeviceNameResponse {
    device_name: String,
}

#[derive(Debug, Clone)]
pub struct AndroidPlaybackBridge {
    handle: PluginHandle<Wry>,
}

impl AndroidPlaybackBridge {
    pub fn new(handle: PluginHandle<Wry>) -> Self {
        Self { handle }
    }

    pub fn play(&self) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("play", ())
            .map_err(|error| format!("Failed to resume the Android playback media: {error}"))
    }

    pub fn load_prepared_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("loadPreparedMedia", payload)
            .map_err(|error| format!("Failed to load the Android playback media: {error}"))
    }

    pub fn prepare_next_media(&self, payload: &AndroidPreparedMediaRequest) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("prepareNextMedia", payload)
            .map_err(|error| format!("Failed to prepare the Android playback media: {error}"))
    }

    pub fn activate_prepared_media(
        &self,
        payload: &AndroidActivatePreparedMediaRequest,
    ) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("activatePreparedMedia", payload)
            .map_err(|error| format!("Failed to activate the Android playback media: {error}"))
    }

    pub fn clear_prepared_media(&self) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("clearPreparedMedia", ())
            .map_err(|error| format!("Failed to clear the Android prepared media: {error}"))
    }

    pub fn update_media_artwork(
        &self,
        payload: &AndroidUpdateMediaArtworkRequest,
    ) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("updateMediaArtwork", payload)
            .map_err(|error| format!("Failed to update the Android media artwork: {error}"))
    }

    pub fn pause(&self) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("pause", ())
            .map_err(|error| format!("Failed to pause the Android playback media: {error}"))
    }

    pub fn stop(&self) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("stop", ())
            .map_err(|error| format!("Failed to stop the Android playback media: {error}"))
    }

    pub fn seek_to(&self, position_ms: u32) -> Result<(), String> {
        self.handle
            .run_mobile_plugin::<()>("seekTo", AndroidSeekPayload { position_ms })
            .map_err(|error| format!("Failed to seek the Android playback media: {error}"))
    }

    pub fn current_position_ms(&self) -> Result<u32, String> {
        let response = self
            .handle
            .run_mobile_plugin::<AndroidPositionResponse>("currentPositionMs", ())
            .map_err(|error| format!("Failed to query the Android playback position: {error}"))?;
        Ok(u32::try_from(response.position_ms).unwrap_or(u32::MAX))
    }

    pub fn default_device_name(&self) -> Result<String, String> {
        let response = self
            .handle
            .run_mobile_plugin::<AndroidDeviceNameResponse>("defaultDeviceName", ())
            .map_err(|error| format!("Failed to query the Android device name: {error}"))?;
        Ok(response.device_name)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidSeekPayload {
    position_ms: u32,
}

#[derive(Debug, Clone)]
pub struct AndroidPlaybackRuntimeState {
    pub bridge: AndroidPlaybackBridge,
    pub event_hub: AndroidPlaybackEventHub,
}

pub fn init() -> TauriPlugin<Wry> {
    PluginBuilder::<Wry>::new("android-playback")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin(PLUGIN_IDENTIFIER, "AndroidPlaybackPlugin")?;
                app.manage(AndroidPlaybackRuntimeState {
                    bridge: AndroidPlaybackBridge::new(handle),
                    event_hub: AndroidPlaybackEventHub::default(),
                });
            }
            let _ = (app, api);
            Ok(())
        })
        .build()
}
