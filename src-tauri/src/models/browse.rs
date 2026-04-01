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
pub struct ArtistIndexItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistIndexesResponse {
    pub artists: Vec<ArtistIndexItem>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArtistIndexesRequest {
    pub music_folder_id: Option<String>,
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
    pub is_directory: bool,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSongsResponse {
    pub album_id: String,
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
