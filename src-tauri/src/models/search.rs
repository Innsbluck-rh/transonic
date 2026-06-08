use serde::{Deserialize, Serialize};

use super::{AlbumListItem, ArtistSummary, SongResponse};

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub artists: Vec<ArtistSummary>,
    pub albums: Vec<AlbumListItem>,
    pub songs: Vec<SongResponse>,
}
