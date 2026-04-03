use std::collections::HashSet;

use async_trait::async_trait;
use opensubsonic_client::{
    api::browsing::{
        BrowsingApi, GetMusicDirectoryRequest, GetMusicDirectoryResponse as ApiDirectoryResponse,
    },
    ApiError, Envelope,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error, normalize_media_type, normalize_text},
        json::{opt_boolish, opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{FolderStructureAlbumSongsRequest, FolderStructureAlbumSongsResponse, SongResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
struct RawDirectory {
    #[serde(default, deserialize_with = "vec_or_single")]
    child: Vec<RawChild>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChild {
    #[serde(deserialize_with = "stringish")]
    id: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    parent: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    path: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    title: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    album: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    album_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    track: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    disc_number: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    year: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    duration: Option<u32>,
    #[serde(default, deserialize_with = "opt_stringish")]
    media_type: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    content_type: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    item_type: Option<String>,
    #[serde(default, deserialize_with = "opt_boolish")]
    is_dir: Option<bool>,
    #[serde(default, deserialize_with = "opt_boolish")]
    is_video: Option<bool>,
}

impl RawChild {
    fn is_directory(&self) -> bool {
        self.is_dir.unwrap_or(false)
    }

    fn is_song(&self) -> bool {
        if self.is_directory() {
            return false;
        }

        if let Some(media_type) = normalize_text(self.media_type.as_deref()) {
            return media_type.eq_ignore_ascii_case("song");
        }

        if self.is_video.unwrap_or(false) {
            return false;
        }

        if let Some(content_type) = normalize_text(self.content_type.as_deref()) {
            if content_type.to_ascii_lowercase().starts_with("audio/") {
                return true;
            }
        }

        if let Some(item_type) = normalize_text(self.item_type.as_deref()) {
            if item_type.eq_ignore_ascii_case("music") {
                return true;
            }
        }

        true
    }

    fn resolved_album_id(&self) -> String {
        normalize_text(self.album_id.as_deref()).unwrap_or_else(|| album_key(self))
    }

    fn into_song(self) -> SongResponse {
        SongResponse {
            id: self.id.clone(),
            parent_id: self.parent,
            path: self.path,
            title: normalize_text(self.title.as_deref()).unwrap_or(self.id),
            album: self.album,
            album_id: self.album_id,
            artist: self.artist,
            artist_id: self.artist_id,
            cover_art_id: self.cover_art,
            track: self.track,
            disc_number: self.disc_number,
            year: self.year,
            duration: self.duration,
            size: None,
            content_type: None,
            suffix: None,
            bit_rate: None,
            genre: None,
            created: None,
            starred: None,
            is_directory: self.is_dir.unwrap_or(false),
            media_type: normalize_media_type(self.media_type),
        }
    }
}

#[async_trait]
pub(crate) trait AlbumSongsApi: Send + Sync {
    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<ApiDirectoryResponse>, ApiError>;
}

#[async_trait]
impl<T> AlbumSongsApi for T
where
    T: BrowsingApi + Send + Sync,
{
    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<ApiDirectoryResponse>, ApiError> {
        BrowsingApi::get_music_directory(self, req).await
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_folder_structure_album_songs(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: FolderStructureAlbumSongsRequest,
) -> Result<FolderStructureAlbumSongsResponse, String> {
    let library_id = payload.library_id.trim();
    if library_id.is_empty() {
        return Err("libraryId is required.".to_string());
    }

    let node_id = payload.node_id.trim();
    if node_id.is_empty() {
        return Err("nodeId is required.".to_string());
    }

    let album_id = payload.album_id.trim();
    if album_id.is_empty() {
        return Err("albumId is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    load_album_songs_from_client(&client, library_id, node_id, album_id)
        .await
        .map_err(format_api_error)
}

fn parse_directory(payload: Value) -> Result<RawDirectory, String> {
    value_as(payload)
        .map_err(|error| format!("Failed to parse the music directory payload: {error}"))
}

pub(crate) async fn load_album_songs_from_client<C>(
    client: &C,
    library_id: &str,
    node_id: &str,
    album_id: &str,
) -> Result<FolderStructureAlbumSongsResponse, ApiError>
where
    C: AlbumSongsApi,
{
    let mut visited_ids = HashSet::new();
    let mut stack = vec![node_id.to_string()];
    let mut songs = Vec::new();

    while let Some(current_id) = stack.pop() {
        if !visited_ids.insert(current_id.clone()) {
            continue;
        }

        let response = client
            .get_music_directory(GetMusicDirectoryRequest { id: current_id })
            .await?;
        let directory = parse_directory(response.payload.directory).map_err(ApiError::Protocol)?;

        let mut directories = Vec::new();
        for child in directory.child {
            if child.is_directory() {
                directories.push(child.id.clone());
                continue;
            }

            if child.is_song() && child.resolved_album_id() == album_id {
                songs.push(child.into_song());
            }
        }

        for directory_id in directories.into_iter().rev() {
            stack.push(directory_id);
        }
    }

    sort_songs_for_queue(&mut songs);

    Ok(FolderStructureAlbumSongsResponse {
        library_id: library_id.to_string(),
        node_id: node_id.to_string(),
        album_id: album_id.to_string(),
        songs,
    })
}

fn sort_songs_for_queue(songs: &mut [SongResponse]) {
    songs.sort_by(|left, right| {
        left.disc_number
            .unwrap_or(u32::MAX)
            .cmp(&right.disc_number.unwrap_or(u32::MAX))
            .then_with(|| {
                left.track
                    .unwrap_or(u32::MAX)
                    .cmp(&right.track.unwrap_or(u32::MAX))
            })
            .then_with(|| {
                left.title
                    .to_ascii_lowercase()
                    .cmp(&right.title.to_ascii_lowercase())
            })
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn album_key(song: &RawChild) -> String {
    if let Some(album_id) = normalize_text(song.album_id.as_deref()) {
        return format!("album_id:{album_id}");
    }

    let album = normalize_text(song.album.as_deref());
    let artist = normalize_text(song.artist.as_deref());

    if let (Some(album), Some(artist), Some(year)) = (album.clone(), artist.clone(), song.year) {
        return format!("album_artist_year:{album}\u{1f}{artist}\u{1f}{year}");
    }

    if let (Some(album), Some(artist)) = (album, artist) {
        return format!("album_artist:{album}\u{1f}{artist}");
    }

    "unknown_album".to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use opensubsonic_client::{ApiError, ResponseMeta, ResponseStatus};
    use serde_json::json;

    use super::{
        load_album_songs_from_client, AlbumSongsApi, ApiDirectoryResponse, GetMusicDirectoryRequest,
    };
    use opensubsonic_client::Envelope;

    #[derive(Debug, Clone)]
    enum MockResponse {
        Ok(serde_json::Value),
    }

    #[derive(Debug, Clone, Default)]
    struct MockAlbumSongsApi {
        directory_responses: HashMap<String, MockResponse>,
        call_log: Arc<Mutex<Vec<String>>>,
    }

    impl MockAlbumSongsApi {
        fn with_directory_response(mut self, id: &str, response: MockResponse) -> Self {
            self.directory_responses.insert(id.to_string(), response);
            self
        }
    }

    #[async_trait]
    impl AlbumSongsApi for MockAlbumSongsApi {
        async fn get_music_directory(
            &self,
            req: GetMusicDirectoryRequest,
        ) -> Result<Envelope<ApiDirectoryResponse>, ApiError> {
            self.call_log
                .lock()
                .unwrap()
                .push(format!("directory:{}", req.id));

            match self
                .directory_responses
                .get(&req.id)
                .cloned()
                .expect("missing mock directory response")
            {
                MockResponse::Ok(directory) => Ok(ok_envelope(ApiDirectoryResponse { directory })),
            }
        }
    }

    fn ok_envelope<T>(payload: T) -> Envelope<T> {
        Envelope {
            meta: ResponseMeta {
                status: ResponseStatus::Ok,
                api_version: "1.16.1".to_string(),
                server_type: Some("mock".to_string()),
                server_version: Some("1.0.0".to_string()),
                open_subsonic: Some(true),
            },
            payload,
        }
    }

    fn folder_song(
        id: &str,
        album: Option<&str>,
        album_id: Option<&str>,
        artist: Option<&str>,
        year: Option<u32>,
        track: Option<u32>,
    ) -> serde_json::Value {
        json!({
            "id": id,
            "title": id,
            "isDir": false,
            "album": album,
            "albumId": album_id,
            "artist": artist,
            "year": year,
            "track": track,
            "mediaType": "song"
        })
    }

    #[tokio::test]
    async fn load_album_songs_filters_by_album_id() {
        let api = MockAlbumSongsApi::default()
            .with_directory_response(
                "root",
                MockResponse::Ok(json!({
                    "id": "root",
                    "child": [
                        folder_song("song-1", Some("Album A"), Some("album-a"), Some("Artist A"), Some(2024), Some(2)),
                        folder_song("song-2", Some("Album B"), Some("album-b"), Some("Artist A"), Some(2024), Some(1))
                    ]
                })),
            );

        let response = load_album_songs_from_client(&api, "library-1", "root", "album-a")
            .await
            .unwrap();

        assert_eq!(response.songs.len(), 1);
        assert_eq!(response.songs[0].id, "song-1");
    }

    #[tokio::test]
    async fn load_album_songs_accepts_computed_album_key() {
        let key = "album_artist_year:Album A\u{1f}Artist A\u{1f}2024";
        let api = MockAlbumSongsApi::default()
            .with_directory_response(
                "root",
                MockResponse::Ok(json!({
                    "id": "root",
                    "child": [
                        folder_song("song-2", Some("Album A"), None, Some("Artist A"), Some(2024), Some(2)),
                        folder_song("song-1", Some("Album A"), None, Some("Artist A"), Some(2024), Some(1))
                    ]
                })),
            );

        let response = load_album_songs_from_client(&api, "library-1", "root", key)
            .await
            .unwrap();

        assert_eq!(response.songs.len(), 2);
        assert_eq!(response.songs[0].id, "song-1");
        assert_eq!(response.songs[1].id, "song-2");
    }

    #[tokio::test]
    async fn load_album_songs_accepts_unknown_album_key() {
        let api = MockAlbumSongsApi::default().with_directory_response(
            "root",
            MockResponse::Ok(json!({
                "id": "root",
                "child": [
                    folder_song("song-1", None, None, None, None, Some(1))
                ]
            })),
        );

        let response = load_album_songs_from_client(&api, "library-1", "root", "unknown_album")
            .await
            .unwrap();

        assert_eq!(response.songs.len(), 1);
        assert_eq!(response.songs[0].id, "song-1");
    }
}
