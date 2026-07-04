use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::OpenSubsonicClient,
    envelope::Envelope,
    error::ApiError,
    types::common::{opt_stringish, opt_u32ish, stringish, vec_or_single, Child},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newer_than: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResponse {
    #[serde(rename = "searchResult")]
    pub search_result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Search2Request {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Search2Response {
    #[serde(rename = "searchResult2")]
    pub search_result2: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Search3Request {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Search3Response {
    #[serde(rename = "searchResult3")]
    pub search_result3: SearchResult3,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult3 {
    #[serde(default, deserialize_with = "vec_or_single")]
    pub artist: Vec<SearchArtist>,
    #[serde(default, deserialize_with = "vec_or_single")]
    pub album: Vec<SearchAlbum>,
    #[serde(default, deserialize_with = "vec_or_single")]
    pub song: Vec<Child>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchArtist {
    #[serde(deserialize_with = "stringish")]
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub album_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub artist_image_url: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub starred: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub music_brainz_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub sort_name: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchAlbum {
    #[serde(deserialize_with = "stringish")]
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub artist_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub song_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub duration: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub play_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub created: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub starred: Option<String>,
}

#[async_trait]
pub trait SearchApi {
    async fn search(&self, req: SearchRequest) -> Result<Envelope<SearchResponse>, ApiError>;
    async fn search2(&self, req: Search2Request) -> Result<Envelope<Search2Response>, ApiError>;
    async fn search3(&self, req: Search3Request) -> Result<Envelope<Search3Response>, ApiError>;
}

#[async_trait]
impl SearchApi for OpenSubsonicClient {
    async fn search(&self, req: SearchRequest) -> Result<Envelope<SearchResponse>, ApiError> {
        self.json_get("search", &req).await
    }

    async fn search2(&self, req: Search2Request) -> Result<Envelope<Search2Response>, ApiError> {
        self.json_get("search2", &req).await
    }

    async fn search3(&self, req: Search3Request) -> Result<Envelope<Search3Response>, ApiError> {
        self.json_get("search3", &req).await
    }
}
