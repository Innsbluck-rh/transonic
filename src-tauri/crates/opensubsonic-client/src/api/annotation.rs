use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    client::OpenSubsonicClient,
    envelope::Envelope,
    error::ApiError,
    types::common::{MediaType, PlaybackState},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StarRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "id")]
    pub song_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "albumId")]
    pub album_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "artistId")]
    pub artist_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StarResponse {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UnstarRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "id")]
    pub song_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "albumId")]
    pub album_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "artistId")]
    pub artist_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UnstarResponse {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetRatingRequest {
    pub id: String,
    pub rating: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SetRatingResponse {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScrobbleRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "id")]
    pub ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScrobbleResponse {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReportPlaybackRequest {
    pub media_id: String,
    pub media_type: MediaType,
    pub position_ms: u64,
    pub state: PlaybackState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_scrobble: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ReportPlaybackResponse {}

#[async_trait]
pub trait AnnotationApi {
    async fn star(&self, req: StarRequest) -> Result<Envelope<StarResponse>, ApiError>;
    async fn unstar(&self, req: UnstarRequest) -> Result<Envelope<UnstarResponse>, ApiError>;
    async fn set_rating(
        &self,
        req: SetRatingRequest,
    ) -> Result<Envelope<SetRatingResponse>, ApiError>;
    async fn scrobble(&self, req: ScrobbleRequest) -> Result<Envelope<ScrobbleResponse>, ApiError>;
    async fn report_playback(
        &self,
        req: ReportPlaybackRequest,
    ) -> Result<Envelope<ReportPlaybackResponse>, ApiError>;
}

#[async_trait]
impl AnnotationApi for OpenSubsonicClient {
    async fn star(&self, req: StarRequest) -> Result<Envelope<StarResponse>, ApiError> {
        self.json_get("star", &req).await
    }

    async fn unstar(&self, req: UnstarRequest) -> Result<Envelope<UnstarResponse>, ApiError> {
        self.json_get("unstar", &req).await
    }

    async fn set_rating(
        &self,
        req: SetRatingRequest,
    ) -> Result<Envelope<SetRatingResponse>, ApiError> {
        self.json_get("setRating", &req).await
    }

    async fn scrobble(&self, req: ScrobbleRequest) -> Result<Envelope<ScrobbleResponse>, ApiError> {
        self.json_get("scrobble", &req).await
    }

    async fn report_playback(
        &self,
        req: ReportPlaybackRequest,
    ) -> Result<Envelope<ReportPlaybackResponse>, ApiError> {
        self.json_get("reportPlayback", &req).await
    }
}
