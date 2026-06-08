use reqwest::Url;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AlbumDisplayMode {
    Grid,
    List,
}

impl Default for AlbumDisplayMode {
    fn default() -> Self {
        Self::Grid
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub album_display_mode: AlbumDisplayMode,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            album_display_mode: AlbumDisplayMode::default(),
        }
    }
}

impl<'de> Deserialize<'de> for AppearanceSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawAppearanceSettings {
            #[serde(default)]
            album_display_mode: Option<AlbumDisplayMode>,
        }

        let raw = RawAppearanceSettings::deserialize(deserializer)?;
        Ok(Self {
            album_display_mode: raw.album_display_mode.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettings {
    pub gapless_playback_enabled: bool,
    pub volume: f32,
    pub stream_mode: PlaybackStreamMode,
    pub transcoding_bitrate_limit: u32,
    pub use_custom_transcoding_codec: bool,
    pub transcoding_codec: PlaybackTranscodingCodec,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            gapless_playback_enabled: true,
            volume: default_volume(),
            stream_mode: PlaybackStreamMode::default(),
            transcoding_bitrate_limit: default_transcoding_bitrate_limit(),
            use_custom_transcoding_codec: false,
            transcoding_codec: PlaybackTranscodingCodec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStreamMode {
    Raw,
    Transcoding,
}

impl Default for PlaybackStreamMode {
    fn default() -> Self {
        Self::Raw
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackTranscodingCodec {
    Auto,
    Mp3,
    Flac,
    Aac,
    Alac,
    Vorbis,
    Opus,
}

impl PlaybackTranscodingCodec {
    pub fn stream_format(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Mp3 => Some("mp3"),
            Self::Flac => Some("flac"),
            Self::Aac => Some("aac"),
            Self::Alac => Some("alac"),
            Self::Vorbis => Some("vorbis"),
            Self::Opus => Some("opus"),
        }
    }
}

impl Default for PlaybackTranscodingCodec {
    fn default() -> Self {
        Self::Auto
    }
}

pub fn effective_transcoding_codec(
    use_custom_transcoding_codec: bool,
    transcoding_codec: PlaybackTranscodingCodec,
) -> PlaybackTranscodingCodec {
    if use_custom_transcoding_codec {
        transcoding_codec
    } else {
        PlaybackTranscodingCodec::Mp3
    }
}

pub fn normalize_transcoding_bitrate_limit(limit: u32) -> u32 {
    if limit > 0 {
        limit
    } else {
        default_transcoding_bitrate_limit()
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
            #[serde(default)]
            volume: Option<f32>,
            #[serde(default)]
            stream_mode: PlaybackStreamMode,
            #[serde(default = "default_transcoding_bitrate_limit")]
            transcoding_bitrate_limit: u32,
            #[serde(default)]
            use_custom_transcoding_codec: bool,
            #[serde(default)]
            transcoding_codec: PlaybackTranscodingCodec,
        }

        let raw = RawPlaybackSettings::deserialize(deserializer)?;
        Ok(Self {
            gapless_playback_enabled: raw
                .gapless_playback_enabled
                .unwrap_or_else(|| raw.prebuffer_strategy.as_deref() == Some("next_track")),
            volume: normalize_volume_option(raw.volume),
            stream_mode: raw.stream_mode,
            transcoding_bitrate_limit: normalize_transcoding_bitrate_limit(
                raw.transcoding_bitrate_limit,
            ),
            use_custom_transcoding_codec: raw.use_custom_transcoding_codec,
            transcoding_codec: raw.transcoding_codec,
        })
    }
}

pub fn normalize_volume(volume: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        default_volume()
    }
}

fn normalize_volume_option(volume: Option<f32>) -> f32 {
    volume.map(normalize_volume).unwrap_or_else(default_volume)
}

fn default_volume() -> f32 {
    1.0
}

fn default_transcoding_bitrate_limit() -> u32 {
    320
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

#[derive(Debug, Clone, Serialize, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub appearance: AppearanceSettings,
    pub playback: PlaybackSettings,
    pub connect: ConnectSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            appearance: AppearanceSettings::default(),
            playback: PlaybackSettings::default(),
            connect: ConnectSettings::default(),
        }
    }
}

impl<'de> Deserialize<'de> for AppSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawAppSettings {
            #[serde(default)]
            appearance: AppearanceSettings,
            #[serde(default)]
            playback: PlaybackSettings,
            #[serde(default)]
            connect: ConnectSettings,
        }

        let raw = RawAppSettings::deserialize(deserializer)?;
        Ok(Self {
            appearance: raw.appearance,
            playback: raw.playback,
            connect: raw.connect,
        })
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
