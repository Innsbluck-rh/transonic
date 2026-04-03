use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtRequest {
    pub cover_art_id: String,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtResponse {
    pub local_path: String,
}
