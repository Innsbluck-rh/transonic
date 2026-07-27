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
    pub use_custom_output: bool,
    /// Whether ASIO devices are offered in the custom output list. Off by default:
    /// ASIO takes exclusive control of the device and bypasses the Windows mixer,
    /// which is not what someone who has not asked for it should get.
    pub use_asio_output: bool,
    pub output_device_id: Option<String>,
    pub stream_mode: PlaybackStreamMode,
    pub metered_network_transcoding_enabled: bool,
    pub transcoding_bitrate_limit: u32,
    pub use_custom_transcoding_codec: bool,
    pub transcoding_codec: PlaybackTranscodingCodec,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            gapless_playback_enabled: true,
            volume: default_volume(),
            use_custom_output: false,
            use_asio_output: false,
            output_device_id: None,
            stream_mode: PlaybackStreamMode::default(),
            metered_network_transcoding_enabled: false,
            transcoding_bitrate_limit: default_transcoding_bitrate_limit(),
            use_custom_transcoding_codec: false,
            transcoding_codec: PlaybackTranscodingCodec::default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCostState {
    Unknown,
    Metered,
    Unmetered,
}

impl Default for NetworkCostState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectivePlaybackStreamSettings {
    pub stream_mode: PlaybackStreamMode,
    pub transcoding_bitrate_limit: u32,
    pub transcoding_codec: PlaybackTranscodingCodec,
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

pub fn resolve_effective_stream_settings(
    playback: &PlaybackSettings,
    network_cost_state: NetworkCostState,
) -> EffectivePlaybackStreamSettings {
    let stream_mode = if playback.metered_network_transcoding_enabled
        && network_cost_state != NetworkCostState::Unmetered
    {
        PlaybackStreamMode::Transcoding
    } else {
        playback.stream_mode
    };

    EffectivePlaybackStreamSettings {
        stream_mode,
        transcoding_bitrate_limit: playback.transcoding_bitrate_limit,
        transcoding_codec: effective_transcoding_codec(
            playback.use_custom_transcoding_codec,
            playback.transcoding_codec,
        ),
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
            use_custom_output: Option<bool>,
            #[serde(default)]
            use_asio_output: bool,
            #[serde(default)]
            output_device_id: Option<String>,
            #[serde(default)]
            stream_mode: PlaybackStreamMode,
            #[serde(default)]
            metered_network_transcoding_enabled: bool,
            #[serde(default = "default_transcoding_bitrate_limit")]
            transcoding_bitrate_limit: u32,
            #[serde(default)]
            use_custom_transcoding_codec: bool,
            #[serde(default)]
            transcoding_codec: PlaybackTranscodingCodec,
        }

        let raw = RawPlaybackSettings::deserialize(deserializer)?;
        let output_device_id = normalize_output_device_id(raw.output_device_id);
        let use_custom_output = raw
            .use_custom_output
            .unwrap_or_else(|| output_device_id.is_some());
        Ok(Self {
            gapless_playback_enabled: raw
                .gapless_playback_enabled
                .unwrap_or_else(|| raw.prebuffer_strategy.as_deref() == Some("next_track")),
            volume: normalize_volume_option(raw.volume),
            use_custom_output,
            use_asio_output: raw.use_asio_output,
            output_device_id,
            stream_mode: raw.stream_mode,
            metered_network_transcoding_enabled: raw.metered_network_transcoding_enabled,
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

pub fn normalize_output_device_id(output_device_id: Option<String>) -> Option<String> {
    output_device_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn effective_output_device_id(playback: &PlaybackSettings) -> Option<&str> {
    let output_device_id = playback
        .use_custom_output
        .then_some(playback.output_device_id.as_deref())
        .flatten()?;
    // A saved ASIO device is only honoured while ASIO output is enabled, so a
    // settings file that still names one after the toggle was turned off does not
    // quietly keep playing through it.
    if !playback.use_asio_output && is_asio_output_device_id(output_device_id) {
        return None;
    }
    Some(output_device_id)
}

/// Whether a saved output device id refers to an ASIO device.
pub fn is_asio_output_device_id(output_device_id: &str) -> bool {
    matches!(
        super::playback::PlaybackOutputHost::from_output_device_id(output_device_id),
        Some(super::playback::PlaybackOutputHost::Asio)
    )
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

#[cfg(test)]
mod tests {
    use super::{
        effective_output_device_id, resolve_effective_stream_settings, NetworkCostState,
        PlaybackSettings, PlaybackStreamMode, PlaybackTranscodingCodec,
    };

    #[test]
    fn effective_stream_settings_use_saved_mode_when_metered_override_is_disabled() {
        let mut playback = PlaybackSettings::default();
        playback.stream_mode = PlaybackStreamMode::Raw;
        playback.metered_network_transcoding_enabled = false;

        let effective = resolve_effective_stream_settings(&playback, NetworkCostState::Metered);

        assert_eq!(effective.stream_mode, PlaybackStreamMode::Raw);
    }

    #[test]
    fn effective_stream_settings_use_transcoding_on_metered_network() {
        let mut playback = PlaybackSettings::default();
        playback.stream_mode = PlaybackStreamMode::Raw;
        playback.metered_network_transcoding_enabled = true;
        playback.transcoding_bitrate_limit = 192;
        playback.use_custom_transcoding_codec = true;
        playback.transcoding_codec = PlaybackTranscodingCodec::Opus;

        let effective = resolve_effective_stream_settings(&playback, NetworkCostState::Metered);

        assert_eq!(effective.stream_mode, PlaybackStreamMode::Transcoding);
        assert_eq!(effective.transcoding_bitrate_limit, 192);
        assert_eq!(effective.transcoding_codec, PlaybackTranscodingCodec::Opus);
    }

    #[test]
    fn effective_stream_settings_treat_unknown_network_as_metered() {
        let mut playback = PlaybackSettings::default();
        playback.stream_mode = PlaybackStreamMode::Raw;
        playback.metered_network_transcoding_enabled = true;

        let effective = resolve_effective_stream_settings(&playback, NetworkCostState::Unknown);

        assert_eq!(effective.stream_mode, PlaybackStreamMode::Transcoding);
    }

    #[test]
    fn effective_stream_settings_keep_raw_on_unmetered_network() {
        let mut playback = PlaybackSettings::default();
        playback.stream_mode = PlaybackStreamMode::Raw;
        playback.metered_network_transcoding_enabled = true;

        let effective = resolve_effective_stream_settings(&playback, NetworkCostState::Unmetered);

        assert_eq!(effective.stream_mode, PlaybackStreamMode::Raw);
    }

    #[test]
    fn effective_output_device_id_uses_saved_id_only_when_custom_output_is_enabled() {
        let mut playback = PlaybackSettings::default();
        playback.output_device_id = Some("output:device-1".to_string());

        assert_eq!(effective_output_device_id(&playback), None);

        playback.use_custom_output = true;

        assert_eq!(
            effective_output_device_id(&playback),
            Some("output:device-1")
        );
    }

    #[test]
    fn effective_output_device_id_ignores_asio_device_while_asio_output_is_disabled() {
        let mut playback = PlaybackSettings::default();
        playback.use_custom_output = true;
        playback.output_device_id = Some("asio:Audient USB Audio ASIO Driver".to_string());

        assert_eq!(effective_output_device_id(&playback), None);

        playback.use_asio_output = true;

        assert_eq!(
            effective_output_device_id(&playback),
            Some("asio:Audient USB Audio ASIO Driver")
        );
    }

    #[test]
    fn effective_output_device_id_keeps_wasapi_device_while_asio_output_is_disabled() {
        let mut playback = PlaybackSettings::default();
        playback.use_custom_output = true;
        playback.output_device_id = Some("wasapi:{0.0.0.00000000}.{device}".to_string());

        assert_eq!(
            effective_output_device_id(&playback),
            Some("wasapi:{0.0.0.00000000}.{device}")
        );
    }
}
