use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{client::OpenSubsonicClient, envelope::Envelope, error::ApiError};

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
    pub search_result3: Value,
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
