use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::OpenSubsonicClient, envelope::Envelope, error::ApiError,
    transport::PreparedBinaryRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bit_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimate_content_length: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HlsRequest {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "bitRate")]
    pub bit_rates: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_track: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetCaptionsRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetCaptionsResponse {
    #[serde(rename = "captions")]
    pub captions: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetCoverArtRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAvatarRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetLyricsRequest {
    pub artist: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetLyricsResponse {
    #[serde(rename = "lyrics")]
    pub lyrics: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetLyricsBySongIdRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetLyricsBySongIdResponse {
    #[serde(rename = "lyricsList")]
    pub lyrics_list: Value,
}

#[async_trait]
pub trait RetrievalApi {
    fn stream(&self, req: StreamRequest) -> Result<PreparedBinaryRequest, ApiError>;
    fn download(&self, req: DownloadRequest) -> Result<PreparedBinaryRequest, ApiError>;
    fn hls(&self, req: HlsRequest) -> Result<PreparedBinaryRequest, ApiError>;
    async fn get_captions(
        &self,
        req: GetCaptionsRequest,
    ) -> Result<Envelope<GetCaptionsResponse>, ApiError>;
    fn get_cover_art(&self, req: GetCoverArtRequest) -> Result<PreparedBinaryRequest, ApiError>;
    fn get_avatar(&self, req: GetAvatarRequest) -> Result<PreparedBinaryRequest, ApiError>;
    async fn get_lyrics(
        &self,
        req: GetLyricsRequest,
    ) -> Result<Envelope<GetLyricsResponse>, ApiError>;
    async fn get_lyrics_by_song_id(
        &self,
        req: GetLyricsBySongIdRequest,
    ) -> Result<Envelope<GetLyricsBySongIdResponse>, ApiError>;
}

#[async_trait]
impl RetrievalApi for OpenSubsonicClient {
    fn stream(&self, req: StreamRequest) -> Result<PreparedBinaryRequest, ApiError> {
        self.binary_get("stream", &req)
    }

    fn download(&self, req: DownloadRequest) -> Result<PreparedBinaryRequest, ApiError> {
        self.binary_get("download", &req)
    }

    fn hls(&self, req: HlsRequest) -> Result<PreparedBinaryRequest, ApiError> {
        self.binary_get("hls", &req)
    }

    async fn get_captions(
        &self,
        req: GetCaptionsRequest,
    ) -> Result<Envelope<GetCaptionsResponse>, ApiError> {
        self.json_get("getCaptions", &req).await
    }

    fn get_cover_art(&self, req: GetCoverArtRequest) -> Result<PreparedBinaryRequest, ApiError> {
        self.binary_get("getCoverArt", &req)
    }

    fn get_avatar(&self, req: GetAvatarRequest) -> Result<PreparedBinaryRequest, ApiError> {
        self.binary_get("getAvatar", &req)
    }

    async fn get_lyrics(
        &self,
        req: GetLyricsRequest,
    ) -> Result<Envelope<GetLyricsResponse>, ApiError> {
        self.json_get("getLyrics", &req).await
    }

    async fn get_lyrics_by_song_id(
        &self,
        req: GetLyricsBySongIdRequest,
    ) -> Result<Envelope<GetLyricsBySongIdResponse>, ApiError> {
        self.json_get("getLyricsBySongId", &req).await
    }
}
