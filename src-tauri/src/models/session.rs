use serde::{Deserialize, Serialize};

use super::{
    auth::{AuthInput, AuthKind},
    settings::{AppSettings, SettingsOrigin},
};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectServerProfileRequest {
    pub profile_id: Option<String>,
    pub display_name: Option<String>,
    pub server_url: String,
    pub auth: AuthInput,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileIdRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicExtension {
    pub name: String,
    pub versions: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityMatrix {
    pub open_subsonic: bool,
    pub raw_extensions: Vec<OpenSubsonicExtension>,
    pub api_key_auth: bool,
    pub index_based_queue: bool,
    pub playback_report: bool,
    pub transcoding: bool,
    pub transcode_offset: bool,
    pub song_lyrics: bool,
}

impl CapabilityMatrix {
    pub fn empty() -> Self {
        Self {
            open_subsonic: false,
            raw_extensions: Vec::new(),
            api_key_auth: false,
            index_based_queue: false,
            playback_report: false,
            transcoding: false,
            transcode_offset: false,
            song_lyrics: false,
        }
    }

    pub fn from_extensions(
        open_subsonic: bool,
        raw_extensions: Vec<OpenSubsonicExtension>,
    ) -> Self {
        let mut matrix = Self::empty();
        matrix.open_subsonic = open_subsonic;
        matrix.api_key_auth = has_extension(&raw_extensions, "apiKeyAuthentication");
        matrix.index_based_queue = has_extension(&raw_extensions, "indexBasedQueue");
        matrix.playback_report = has_extension(&raw_extensions, "playbackReport");
        matrix.transcoding = has_extension(&raw_extensions, "transcoding");
        matrix.transcode_offset = has_extension(&raw_extensions, "transcodeOffset");
        matrix.song_lyrics = has_extension(&raw_extensions, "songLyrics");
        matrix.raw_extensions = raw_extensions;
        matrix
    }
}

fn has_extension(extensions: &[OpenSubsonicExtension], expected_name: &str) -> bool {
    extensions
        .iter()
        .any(|extension| extension.name == expected_name)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum LastConnectionState {
    Never,
    Ok,
    Offline,
    ReauthRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedProfileSummary {
    pub profile_id: String,
    pub display_name: String,
    pub normalized_server_url: String,
    pub server_url_host: String,
    pub server_url_port: Option<u16>,
    pub server_url_path: Option<String>,
    pub auth_kind: AuthKind,
    pub username: String,
    pub last_connection_state: LastConnectionState,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSession {
    pub profile_id: String,
    pub normalized_server_url: String,
    pub username: String,
    pub auth_kind: AuthKind,
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub capability_matrix: CapabilityMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCapabilities {
    pub gapless_playback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RestoreStatus {
    None,
    Restored,
    NetworkError,
    ConnectionError,
    ReauthRequired,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub profiles: Vec<SavedProfileSummary>,
    pub active_session: Option<ActiveSession>,
    pub restore_status: RestoreStatus,
    pub message: Option<String>,
    pub playback_capabilities: PlaybackCapabilities,
    pub settings: AppSettings,
    pub settings_origin: SettingsOrigin,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConnectServerProfileResult {
    Connected {
        #[serde(rename = "activeSession")]
        active_session: ActiveSession,
        profiles: Vec<SavedProfileSummary>,
    },
    AuthError {
        message: String,
        code: Option<u32>,
        #[serde(rename = "helpUrl")]
        help_url: Option<String>,
        #[serde(rename = "activeSession")]
        active_session: Option<ActiveSession>,
        profiles: Vec<SavedProfileSummary>,
    },
    NetworkError {
        message: String,
        #[serde(rename = "activeSession")]
        active_session: Option<ActiveSession>,
        profiles: Vec<SavedProfileSummary>,
    },
    ServerError {
        message: String,
        code: Option<u32>,
        #[serde(rename = "helpUrl")]
        help_url: Option<String>,
        #[serde(rename = "activeSession")]
        active_session: Option<ActiveSession>,
        profiles: Vec<SavedProfileSummary>,
    },
    UnsupportedAuth {
        message: String,
        #[serde(rename = "activeSession")]
        active_session: Option<ActiveSession>,
        profiles: Vec<SavedProfileSummary>,
    },
}
