use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{client::OpenSubsonicClient, envelope::Envelope, error::ApiError};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GetMusicFoldersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetMusicFoldersResponse {
    #[serde(rename = "musicFolders")]
    pub music_folders: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetIndexesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_modified_since: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetIndexesResponse {
    #[serde(rename = "indexes")]
    pub indexes: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetMusicDirectoryRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetMusicDirectoryResponse {
    #[serde(rename = "directory")]
    pub directory: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GetGenresRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetGenresResponse {
    #[serde(rename = "genres")]
    pub genres: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetArtistsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetArtistsResponse {
    #[serde(rename = "artists")]
    pub artists: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetArtistRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetArtistResponse {
    #[serde(rename = "artist")]
    pub artist: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumResponse {
    #[serde(rename = "album")]
    pub album: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSongRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSongResponse {
    #[serde(rename = "song")]
    pub song: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetArtistInfoRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_not_present: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetArtistInfoResponse {
    #[serde(rename = "artistInfo")]
    pub artist_info: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetArtistInfo2Request {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_not_present: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetArtistInfo2Response {
    #[serde(rename = "artistInfo2")]
    pub artist_info2: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumInfoRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumInfoResponse {
    #[serde(rename = "albumInfo")]
    pub album_info: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumInfo2Request {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetAlbumInfo2Response {
    #[serde(rename = "albumInfo2")]
    pub album_info2: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSimilarSongsRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSimilarSongsResponse {
    #[serde(rename = "similarSongs")]
    pub similar_songs: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSimilarSongs2Request {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetSimilarSongs2Response {
    #[serde(rename = "similarSongs2")]
    pub similar_songs2: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetTopSongsRequest {
    pub artist: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetTopSongsResponse {
    #[serde(rename = "topSongs")]
    pub top_songs: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GetVideosRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetVideosResponse {
    #[serde(rename = "videos")]
    pub videos: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetVideoInfoRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetVideoInfoResponse {
    #[serde(rename = "videoInfo")]
    pub video_info: Value,
}

#[async_trait]
pub trait BrowsingApi {
    async fn get_music_folders(
        &self,
        req: GetMusicFoldersRequest,
    ) -> Result<Envelope<GetMusicFoldersResponse>, ApiError>;
    async fn get_indexes(
        &self,
        req: GetIndexesRequest,
    ) -> Result<Envelope<GetIndexesResponse>, ApiError>;
    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<GetMusicDirectoryResponse>, ApiError>;
    async fn get_genres(
        &self,
        req: GetGenresRequest,
    ) -> Result<Envelope<GetGenresResponse>, ApiError>;
    async fn get_artists(
        &self,
        req: GetArtistsRequest,
    ) -> Result<Envelope<GetArtistsResponse>, ApiError>;
    async fn get_artist(
        &self,
        req: GetArtistRequest,
    ) -> Result<Envelope<GetArtistResponse>, ApiError>;
    async fn get_album(
        &self,
        req: GetAlbumRequest,
    ) -> Result<Envelope<GetAlbumResponse>, ApiError>;
    async fn get_song(&self, req: GetSongRequest) -> Result<Envelope<GetSongResponse>, ApiError>;
    async fn get_artist_info(
        &self,
        req: GetArtistInfoRequest,
    ) -> Result<Envelope<GetArtistInfoResponse>, ApiError>;
    async fn get_artist_info2(
        &self,
        req: GetArtistInfo2Request,
    ) -> Result<Envelope<GetArtistInfo2Response>, ApiError>;
    async fn get_album_info(
        &self,
        req: GetAlbumInfoRequest,
    ) -> Result<Envelope<GetAlbumInfoResponse>, ApiError>;
    async fn get_album_info2(
        &self,
        req: GetAlbumInfo2Request,
    ) -> Result<Envelope<GetAlbumInfo2Response>, ApiError>;
    async fn get_similar_songs(
        &self,
        req: GetSimilarSongsRequest,
    ) -> Result<Envelope<GetSimilarSongsResponse>, ApiError>;
    async fn get_similar_songs2(
        &self,
        req: GetSimilarSongs2Request,
    ) -> Result<Envelope<GetSimilarSongs2Response>, ApiError>;
    async fn get_top_songs(
        &self,
        req: GetTopSongsRequest,
    ) -> Result<Envelope<GetTopSongsResponse>, ApiError>;
    async fn get_videos(
        &self,
        req: GetVideosRequest,
    ) -> Result<Envelope<GetVideosResponse>, ApiError>;
    async fn get_video_info(
        &self,
        req: GetVideoInfoRequest,
    ) -> Result<Envelope<GetVideoInfoResponse>, ApiError>;
}

#[async_trait]
impl BrowsingApi for OpenSubsonicClient {
    async fn get_music_folders(
        &self,
        req: GetMusicFoldersRequest,
    ) -> Result<Envelope<GetMusicFoldersResponse>, ApiError> {
        self.json_get("getMusicFolders", &req).await
    }

    async fn get_indexes(
        &self,
        req: GetIndexesRequest,
    ) -> Result<Envelope<GetIndexesResponse>, ApiError> {
        self.json_get("getIndexes", &req).await
    }

    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<GetMusicDirectoryResponse>, ApiError> {
        self.json_get("getMusicDirectory", &req).await
    }

    async fn get_genres(
        &self,
        req: GetGenresRequest,
    ) -> Result<Envelope<GetGenresResponse>, ApiError> {
        self.json_get("getGenres", &req).await
    }

    async fn get_artists(
        &self,
        req: GetArtistsRequest,
    ) -> Result<Envelope<GetArtistsResponse>, ApiError> {
        self.json_get("getArtists", &req).await
    }

    async fn get_artist(
        &self,
        req: GetArtistRequest,
    ) -> Result<Envelope<GetArtistResponse>, ApiError> {
        self.json_get("getArtist", &req).await
    }

    async fn get_album(
        &self,
        req: GetAlbumRequest,
    ) -> Result<Envelope<GetAlbumResponse>, ApiError> {
        self.json_get("getAlbum", &req).await
    }

    async fn get_song(&self, req: GetSongRequest) -> Result<Envelope<GetSongResponse>, ApiError> {
        self.json_get("getSong", &req).await
    }

    async fn get_artist_info(
        &self,
        req: GetArtistInfoRequest,
    ) -> Result<Envelope<GetArtistInfoResponse>, ApiError> {
        self.json_get("getArtistInfo", &req).await
    }

    async fn get_artist_info2(
        &self,
        req: GetArtistInfo2Request,
    ) -> Result<Envelope<GetArtistInfo2Response>, ApiError> {
        self.json_get("getArtistInfo2", &req).await
    }

    async fn get_album_info(
        &self,
        req: GetAlbumInfoRequest,
    ) -> Result<Envelope<GetAlbumInfoResponse>, ApiError> {
        self.json_get("getAlbumInfo", &req).await
    }

    async fn get_album_info2(
        &self,
        req: GetAlbumInfo2Request,
    ) -> Result<Envelope<GetAlbumInfo2Response>, ApiError> {
        self.json_get("getAlbumInfo2", &req).await
    }

    async fn get_similar_songs(
        &self,
        req: GetSimilarSongsRequest,
    ) -> Result<Envelope<GetSimilarSongsResponse>, ApiError> {
        self.json_get("getSimilarSongs", &req).await
    }

    async fn get_similar_songs2(
        &self,
        req: GetSimilarSongs2Request,
    ) -> Result<Envelope<GetSimilarSongs2Response>, ApiError> {
        self.json_get("getSimilarSongs2", &req).await
    }

    async fn get_top_songs(
        &self,
        req: GetTopSongsRequest,
    ) -> Result<Envelope<GetTopSongsResponse>, ApiError> {
        self.json_get("getTopSongs", &req).await
    }

    async fn get_videos(
        &self,
        req: GetVideosRequest,
    ) -> Result<Envelope<GetVideosResponse>, ApiError> {
        self.json_get("getVideos", &req).await
    }

    async fn get_video_info(
        &self,
        req: GetVideoInfoRequest,
    ) -> Result<Envelope<GetVideoInfoResponse>, ApiError> {
        self.json_get("getVideoInfo", &req).await
    }
}
