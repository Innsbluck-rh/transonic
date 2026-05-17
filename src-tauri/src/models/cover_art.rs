use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtRequest {
    pub profile_id: String,
    pub cover_art_id: String,
    pub size: Option<u32>,
    #[serde(default)]
    pub cached_fallback_sizes: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtResponse {
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtCacheStatus {
    pub profile_id: Option<String>,
    pub cache_dir: String,
    pub entry_count: u32,
    pub file_count: u32,
    pub total_bytes: f64,
    pub ttl_seconds: u32,
}
