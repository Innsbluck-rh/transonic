use reqwest::Url;
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_use_subsonic_server_host")]
    pub use_subsonic_server_host: bool,
    #[serde(default)]
    pub connect_server_host: Option<String>,
    #[serde(default = "default_connect_server_port")]
    pub connect_server_port: u16,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub allow_insecure_connect_server: bool,
}

impl Default for ConnectSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            use_subsonic_server_host: default_use_subsonic_server_host(),
            connect_server_host: None,
            connect_server_port: default_connect_server_port(),
            device_id: String::new(),
            device_name: None,
            allow_insecure_connect_server: false,
        }
    }
}

impl<'de> Deserialize<'de> for ConnectSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawConnectSettings {
            #[serde(default)]
            enabled: bool,
            #[serde(default = "default_use_subsonic_server_host")]
            use_subsonic_server_host: bool,
            #[serde(default)]
            connect_server_host: Option<String>,
            #[serde(default = "default_connect_server_port")]
            connect_server_port: u16,
            #[serde(default)]
            server_url: Option<String>,
            #[serde(default)]
            device_id: String,
            #[serde(default)]
            device_name: Option<String>,
            #[serde(default)]
            allow_insecure_connect_server: bool,
        }

        let raw = RawConnectSettings::deserialize(deserializer)?;
        let legacy = raw.server_url.as_deref().and_then(split_url_host_and_port);
        let connect_server_host = raw
            .connect_server_host
            .filter(|value| !value.trim().is_empty())
            .or_else(|| legacy.as_ref().map(|(host, _)| host.clone()));
        let connect_server_port = legacy
            .and_then(|(_, port)| port)
            .unwrap_or(raw.connect_server_port);

        Ok(Self {
            enabled: raw.enabled,
            use_subsonic_server_host: raw.use_subsonic_server_host,
            connect_server_host,
            connect_server_port,
            device_id: raw.device_id,
            device_name: raw.device_name,
            allow_insecure_connect_server: raw.allow_insecure_connect_server,
        })
    }
}

fn split_url_host_and_port(raw_url: &str) -> Option<(String, Option<u16>)> {
    let mut parsed = Url::parse(raw_url).ok()?;
    let port = parsed.port();
    parsed.set_path("");
    parsed.set_query(None);
    parsed.set_fragment(None);
    let _ = parsed.set_port(None);
    Some((parsed.to_string().trim_end_matches('/').to_string(), port))
}

fn default_use_subsonic_server_host() -> bool {
    true
}

fn default_connect_server_port() -> u16 {
    4747
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub playback: PlaybackSettings,
    pub connect: ConnectSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            playback: PlaybackSettings::default(),
            connect: ConnectSettings::default(),
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
