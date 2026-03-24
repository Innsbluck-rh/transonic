use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::OpenSubsonicClient, envelope::Envelope, error::ApiError, types::common::AlbumListType,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetAlbumListRequest {
    #[serde(rename = "type")]
    pub list_type: AlbumListType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumListResponse {
    #[serde(rename = "albumList")]
    pub album_list: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetAlbumList2Request {
    #[serde(rename = "type")]
    pub list_type: AlbumListType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumList2Response {
    #[serde(rename = "albumList2")]
    pub album_list2: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetRandomSongsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetRandomSongsResponse {
    #[serde(rename = "randomSongs")]
    pub random_songs: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetSongsByGenreRequest {
    pub genre: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSongsByGenreResponse {
    #[serde(rename = "songsByGenre")]
    pub songs_by_genre: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GetNowPlayingRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetNowPlayingResponse {
    #[serde(rename = "nowPlaying")]
    pub now_playing: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetStarredRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetStarredResponse {
    #[serde(rename = "starred")]
    pub starred: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetStarred2Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetStarred2Response {
    #[serde(rename = "starred2")]
    pub starred2: Value,
}

#[async_trait]
pub trait ListsApi {
    async fn get_album_list(
        &self,
        req: GetAlbumListRequest,
    ) -> Result<Envelope<GetAlbumListResponse>, ApiError>;
    async fn get_album_list2(
        &self,
        req: GetAlbumList2Request,
    ) -> Result<Envelope<GetAlbumList2Response>, ApiError>;
    async fn get_random_songs(
        &self,
        req: GetRandomSongsRequest,
    ) -> Result<Envelope<GetRandomSongsResponse>, ApiError>;
    async fn get_songs_by_genre(
        &self,
        req: GetSongsByGenreRequest,
    ) -> Result<Envelope<GetSongsByGenreResponse>, ApiError>;
    async fn get_now_playing(
        &self,
        req: GetNowPlayingRequest,
    ) -> Result<Envelope<GetNowPlayingResponse>, ApiError>;
    async fn get_starred(
        &self,
        req: GetStarredRequest,
    ) -> Result<Envelope<GetStarredResponse>, ApiError>;
    async fn get_starred2(
        &self,
        req: GetStarred2Request,
    ) -> Result<Envelope<GetStarred2Response>, ApiError>;
}

#[async_trait]
impl ListsApi for OpenSubsonicClient {
    async fn get_album_list(
        &self,
        req: GetAlbumListRequest,
    ) -> Result<Envelope<GetAlbumListResponse>, ApiError> {
        self.json_get("getAlbumList", &req).await
    }

    async fn get_album_list2(
        &self,
        req: GetAlbumList2Request,
    ) -> Result<Envelope<GetAlbumList2Response>, ApiError> {
        self.json_get("getAlbumList2", &req).await
    }

    async fn get_random_songs(
        &self,
        req: GetRandomSongsRequest,
    ) -> Result<Envelope<GetRandomSongsResponse>, ApiError> {
        self.json_get("getRandomSongs", &req).await
    }

    async fn get_songs_by_genre(
        &self,
        req: GetSongsByGenreRequest,
    ) -> Result<Envelope<GetSongsByGenreResponse>, ApiError> {
        self.json_get("getSongsByGenre", &req).await
    }

    async fn get_now_playing(
        &self,
        req: GetNowPlayingRequest,
    ) -> Result<Envelope<GetNowPlayingResponse>, ApiError> {
        self.json_get("getNowPlaying", &req).await
    }

    async fn get_starred(
        &self,
        req: GetStarredRequest,
    ) -> Result<Envelope<GetStarredResponse>, ApiError> {
        self.json_get("getStarred", &req).await
    }

    async fn get_starred2(
        &self,
        req: GetStarred2Request,
    ) -> Result<Envelope<GetStarred2Response>, ApiError> {
        self.json_get("getStarred2", &req).await
    }
}
