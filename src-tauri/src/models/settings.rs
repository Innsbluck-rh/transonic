use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettings {
    pub gapless_playback_enabled: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            gapless_playback_enabled: false,
        }
    }
}

impl<'de> Deserialize<'de> for PlaybackSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawPlaybackSettings {
            #[serde(default, alias = "prebufferEnabled")]
            gapless_playback_enabled: Option<bool>,
            #[serde(default)]
            prebuffer_strategy: Option<String>,
        }

        let raw = RawPlaybackSettings::deserialize(deserializer)?;
        Ok(Self {
            gapless_playback_enabled: raw
                .gapless_playback_enabled
                .unwrap_or_else(|| raw.prebuffer_strategy.as_deref() == Some("next_track")),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub playback: PlaybackSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            playback: PlaybackSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SettingsOrigin {
    Default,
    Stored,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateRequest {
    pub settings: AppSettings,
}
