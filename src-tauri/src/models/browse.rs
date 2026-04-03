use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicFolderSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicFoldersResponse {
    pub music_folders: Vec<MusicFolderSummary>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureRootsRequest {
    pub library_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FolderStructureSource {
    Directory,
    Indexes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureRootNode {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureRootsResponse {
    pub library_id: String,
    pub library_name: String,
    pub source: FolderStructureSource,
    pub root_nodes: Vec<FolderStructureRootNode>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureAlbumsRequest {
    pub library_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureAlbumItem {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub cover_art_id: Option<String>,
    pub year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureAlbumsResponse {
    pub library_id: String,
    pub node_id: String,
    pub node_name: String,
    pub albums: Vec<FolderStructureAlbumItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub cover_art_id: Option<String>,
    pub album_count: Option<u32>,
    pub artist_image_url: Option<String>,
    pub starred: Option<String>,
    pub music_brainz_id: Option<String>,
    pub sort_name: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistGroup {
    pub name: String,
    pub artists: Vec<ArtistSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistsResponse {
    pub ignored_articles: Option<String>,
    pub indexes: Vec<ArtistGroup>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistsRequest {
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistAlbum {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub name: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art_id: Option<String>,
    pub song_count: Option<u32>,
    pub duration: Option<u32>,
    pub play_count: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub starred: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistResponse {
    pub id: String,
    pub name: String,
    pub cover_art_id: Option<String>,
    pub album_count: Option<u32>,
    pub artist_image_url: Option<String>,
    pub starred: Option<String>,
    pub music_brainz_id: Option<String>,
    pub sort_name: Option<String>,
    pub roles: Vec<String>,
    pub albums: Vec<ArtistAlbum>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo2Request {
    pub id: String,
    pub count: Option<u32>,
    pub include_not_present: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo2Response {
    pub biography: Option<String>,
    pub music_brainz_id: Option<String>,
    pub last_fm_url: Option<String>,
    pub small_image_url: Option<String>,
    pub medium_image_url: Option<String>,
    pub large_image_url: Option<String>,
    pub similar_artists: Vec<ArtistSummary>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicDirectoryRequest {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SongRequest {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSongsRequest {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureAlbumSongsRequest {
    pub library_id: String,
    pub node_id: String,
    pub album_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicDirectoryChild {
    pub id: String,
    pub parent_id: Option<String>,
    pub path: Option<String>,
    pub title: String,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art_id: Option<String>,
    pub track: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    #[specta(type = Option<f64>)]
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub suffix: Option<String>,
    pub bit_rate: Option<u32>,
    pub genre: Option<String>,
    pub created: Option<String>,
    pub starred: Option<String>,
    pub is_directory: bool,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicDirectoryResponse {
    pub id: String,
    pub name: String,
    pub children: Vec<MusicDirectoryChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SongResponse {
    pub id: String,
    pub parent_id: Option<String>,
    pub path: Option<String>,
    pub title: String,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art_id: Option<String>,
    pub track: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub duration: Option<u32>,
    #[specta(type = Option<f64>)]
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub suffix: Option<String>,
    pub bit_rate: Option<u32>,
    pub genre: Option<String>,
    pub created: Option<String>,
    pub starred: Option<String>,
    pub is_directory: bool,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSongsResponse {
    pub id: String,
    pub name: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art_id: Option<String>,
    pub song_count: Option<u32>,
    pub duration: Option<u32>,
    pub play_count: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub created: Option<String>,
    pub starred: Option<String>,
    pub songs: Vec<SongResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderStructureAlbumSongsResponse {
    pub library_id: String,
    pub node_id: String,
    pub album_id: String,
    pub songs: Vec<SongResponse>,
}
