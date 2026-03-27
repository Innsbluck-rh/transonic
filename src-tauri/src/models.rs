use opensubsonic_client::{api::lists::GetAlbumList2Request, AlbumListType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    Password,
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthInput {
    Password {
        username: String,
        password: String,
    },
    ApiKey {
        #[serde(rename = "apiKey")]
        api_key: String,
    },
}

impl AuthInput {
    pub fn auth_kind(&self) -> AuthKind {
        match self {
            Self::Password { .. } => AuthKind::Password,
            Self::ApiKey { .. } => AuthKind::ApiKey,
        }
    }

    pub fn secret(&self) -> &str {
        match self {
            Self::Password { password, .. } => password,
            Self::ApiKey { api_key } => api_key,
        }
    }

    pub fn restored(auth_kind: AuthKind, username: String, secret: String) -> Self {
        match auth_kind {
            AuthKind::Password => Self::Password {
                username,
                password: secret,
            },
            AuthKind::ApiKey => Self::ApiKey { api_key: secret },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectServerProfileRequest {
    pub profile_id: Option<String>,
    pub display_name: Option<String>,
    pub server_url: String,
    pub auth: AuthInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileIdRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlbumListContext {
    Random,
    Newest,
    Frequent,
    Recent,
    Starred,
    AlphabeticalByName,
    AlphabeticalByArtist,
    ByYear,
    ByGenre,
}

impl AlbumListContext {
    pub fn to_open_subsonic_type(&self) -> AlbumListType {
        match self {
            Self::Random => AlbumListType::Random,
            Self::Newest => AlbumListType::Newest,
            Self::Frequent => AlbumListType::Frequent,
            Self::Recent => AlbumListType::Recent,
            Self::Starred => AlbumListType::Starred,
            Self::AlphabeticalByName => AlbumListType::AlphabeticalByName,
            Self::AlphabeticalByArtist => AlbumListType::AlphabeticalByArtist,
            Self::ByYear => AlbumListType::ByYear,
            Self::ByGenre => AlbumListType::ByGenre,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListRequest {
    pub context: AlbumListContext,
    pub size: Option<u32>,
    pub offset: Option<u32>,
    pub from_year: Option<u32>,
    pub to_year: Option<u32>,
    pub genre: Option<String>,
    pub music_folder_id: Option<String>,
}

impl AlbumListRequest {
    pub fn to_open_subsonic_request(&self) -> Result<GetAlbumList2Request, String> {
        match self.context {
            AlbumListContext::ByYear if self.from_year.is_none() || self.to_year.is_none() => {
                Err("The by_year album context requires both fromYear and toYear.".to_string())
            }
            AlbumListContext::ByGenre if self.genre.as_deref().unwrap_or("").trim().is_empty() => {
                Err("The by_genre album context requires a non-empty genre.".to_string())
            }
            _ => Ok(GetAlbumList2Request {
                list_type: self.context.to_open_subsonic_type(),
                size: self.size,
                offset: self.offset,
                from_year: self.from_year,
                to_year: self.to_year,
                genre: self.genre.clone(),
                music_folder_id: self.music_folder_id.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListItem {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub cover_art_id: Option<String>,
    pub year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListResponse {
    pub context: AlbumListContext,
    pub albums: Vec<AlbumListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MusicFolderSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MusicFoldersResponse {
    pub music_folders: Vec<MusicFolderSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtistIndexItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtistIndexesResponse {
    pub artists: Vec<ArtistIndexItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistIndexesRequest {
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicDirectoryRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MusicDirectoryChild {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub cover_art_id: Option<String>,
    pub year: Option<u32>,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MusicDirectoryResponse {
    pub id: String,
    pub name: String,
    pub children: Vec<MusicDirectoryChild>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtRequest {
    pub cover_art_id: String,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtResponse {
    pub data_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicExtension {
    pub name: String,
    pub versions: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LastConnectionState {
    Never,
    Ok,
    Offline,
    ReauthRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedProfileSummary {
    pub profile_id: String,
    pub display_name: String,
    pub normalized_server_url: String,
    pub auth_kind: AuthKind,
    pub username: String,
    pub last_connection_state: LastConnectionState,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreStatus {
    None,
    Restored,
    Offline,
    ReauthRequired,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub profiles: Vec<SavedProfileSummary>,
    pub active_session: Option<ActiveSession>,
    pub restore_status: RestoreStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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
