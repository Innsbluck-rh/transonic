use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{client::OpenSubsonicClient, envelope::Envelope, error::ApiError};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GetPlaylistsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetPlaylistsResponse {
    #[serde(rename = "playlists")]
    pub playlists: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetPlaylistRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetPlaylistResponse {
    #[serde(rename = "playlist")]
    pub playlist: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlaylistRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "songId")]
    pub song_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CreatePlaylistResponse {
    #[serde(default, rename = "playlist")]
    pub playlist: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlaylistRequest {
    pub playlist_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "songIdToAdd")]
    pub song_ids_to_add: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "songIndexToRemove"
    )]
    pub song_indexes_to_remove: Vec<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UpdatePlaylistResponse {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeletePlaylistRequest {
    pub id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DeletePlaylistResponse {}

#[async_trait]
pub trait PlaylistsApi {
    async fn get_playlists(
        &self,
        req: GetPlaylistsRequest,
    ) -> Result<Envelope<GetPlaylistsResponse>, ApiError>;
    async fn get_playlist(
        &self,
        req: GetPlaylistRequest,
    ) -> Result<Envelope<GetPlaylistResponse>, ApiError>;
    async fn create_playlist(
        &self,
        req: CreatePlaylistRequest,
    ) -> Result<Envelope<CreatePlaylistResponse>, ApiError>;
    async fn update_playlist(
        &self,
        req: UpdatePlaylistRequest,
    ) -> Result<Envelope<UpdatePlaylistResponse>, ApiError>;
    async fn delete_playlist(
        &self,
        req: DeletePlaylistRequest,
    ) -> Result<Envelope<DeletePlaylistResponse>, ApiError>;
}

#[async_trait]
impl PlaylistsApi for OpenSubsonicClient {
    async fn get_playlists(
        &self,
        req: GetPlaylistsRequest,
    ) -> Result<Envelope<GetPlaylistsResponse>, ApiError> {
        self.json_get("getPlaylists", &req).await
    }

    async fn get_playlist(
        &self,
        req: GetPlaylistRequest,
    ) -> Result<Envelope<GetPlaylistResponse>, ApiError> {
        self.json_get("getPlaylist", &req).await
    }

    async fn create_playlist(
        &self,
        req: CreatePlaylistRequest,
    ) -> Result<Envelope<CreatePlaylistResponse>, ApiError> {
        self.json_get("createPlaylist", &req).await
    }

    async fn update_playlist(
        &self,
        req: UpdatePlaylistRequest,
    ) -> Result<Envelope<UpdatePlaylistResponse>, ApiError> {
        self.json_get("updatePlaylist", &req).await
    }

    async fn delete_playlist(
        &self,
        req: DeletePlaylistRequest,
    ) -> Result<Envelope<DeletePlaylistResponse>, ApiError> {
        self.json_get("deletePlaylist", &req).await
    }
}
