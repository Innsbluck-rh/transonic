use std::collections::{HashMap, HashSet};

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
        common::{client, format_api_error, normalize_text},
        json::{opt_boolish, opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{
        FolderStructureAlbumItem, FolderStructureAlbumsRequest, FolderStructureAlbumsResponse,
    },
    ActiveSessionState,
};

const UNKNOWN_ALBUM_NAME: &str = "Unknown Album";

#[derive(Debug, Clone, Deserialize)]
struct RawDirectory {
    name: String,
    #[serde(default, deserialize_with = "vec_or_single")]
    child: Vec<RawChild>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChild {
    #[serde(deserialize_with = "stringish")]
    id: String,
    album: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    album_id: Option<String>,
    artist: Option<String>,
    cover_art: Option<String>,
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
    #[serde(default, deserialize_with = "opt_u32ish")]
    year: Option<u32>,
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

    fn song(&self) -> SongRow {
        SongRow {
            album: self.album.clone(),
            album_id: self.album_id.clone(),
            artist: self.artist.clone(),
            cover_art_id: self.cover_art.clone(),
            year: self.year,
        }
    }
}

#[derive(Debug, Clone)]
struct SongRow {
    album: Option<String>,
    album_id: Option<String>,
    artist: Option<String>,
    cover_art_id: Option<String>,
    year: Option<u32>,
}

#[async_trait]
trait AlbumsApi: Send + Sync {
    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<ApiDirectoryResponse>, ApiError>;
}

#[async_trait]
impl<T> AlbumsApi for T
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
pub async fn get_folder_structure_albums(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: FolderStructureAlbumsRequest,
) -> Result<FolderStructureAlbumsResponse, String> {
    let library_id = payload.library_id.trim();
    if library_id.is_empty() {
        return Err("libraryId is required.".to_string());
    }

    let node_id = payload.node_id.trim();
    if node_id.is_empty() {
        return Err("nodeId is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    load_albums(&client, library_id, node_id)
        .await
        .map_err(format_api_error)
}

fn parse_directory(payload: Value) -> Result<RawDirectory, String> {
    value_as(payload)
        .map_err(|error| format!("Failed to parse the music directory payload: {error}"))
}

async fn load_albums<C>(
    client: &C,
    library_id: &str,
    node_id: &str,
) -> Result<FolderStructureAlbumsResponse, ApiError>
where
    C: AlbumsApi,
{
    let mut visited_ids = HashSet::new();
    let mut stack = vec![node_id.to_string()];
    let mut node_name: Option<String> = None;
    let mut songs = Vec::new();

    while let Some(current_id) = stack.pop() {
        if !visited_ids.insert(current_id.clone()) {
            continue;
        }

        let response = client
            .get_music_directory(GetMusicDirectoryRequest { id: current_id })
            .await?;
        let directory = parse_directory(response.payload.directory).map_err(ApiError::Protocol)?;

        if node_name.is_none() {
            node_name = Some(directory.name.clone());
        }

        let mut directories = Vec::new();
        for child in directory.child {
            if child.is_directory() {
                directories.push(child.id.clone());
                continue;
            }

            if child.is_song() {
                songs.push(child.song());
            }
        }

        for directory_id in directories.into_iter().rev() {
            stack.push(directory_id);
        }
    }

    Ok(FolderStructureAlbumsResponse {
        library_id: library_id.to_string(),
        node_id: node_id.to_string(),
        node_name: node_name.unwrap_or_else(|| "[Unknown]".to_string()),
        albums: aggregate_albums(songs),
    })
}

fn aggregate_albums(songs: Vec<SongRow>) -> Vec<FolderStructureAlbumItem> {
    #[derive(Debug)]
    struct AlbumGroup {
        id: String,
        name: String,
        artist_counts: HashMap<String, usize>,
        artist_order: Vec<String>,
        cover_art_id: Option<String>,
        year: Option<u32>,
    }

    impl AlbumGroup {
        fn new(id: String, name: String) -> Self {
            Self {
                id,
                name,
                artist_counts: HashMap::new(),
                artist_order: Vec::new(),
                cover_art_id: None,
                year: None,
            }
        }

        fn absorb(&mut self, song: &SongRow) {
            if let Some(artist) = normalize_text(song.artist.as_deref()) {
                if !self.artist_counts.contains_key(&artist) {
                    self.artist_order.push(artist.clone());
                }
                *self.artist_counts.entry(artist).or_insert(0) += 1;
            }

            if self.cover_art_id.is_none() {
                self.cover_art_id = normalize_text(song.cover_art_id.as_deref());
            }

            if self.year.is_none() {
                self.year = song.year;
            }
        }

        fn artist(&self) -> Option<String> {
            let mut best_artist: Option<&str> = None;
            let mut best_count = 0usize;

            for artist in &self.artist_order {
                let count = self.artist_counts.get(artist).copied().unwrap_or_default();
                if count > best_count {
                    best_artist = Some(artist.as_str());
                    best_count = count;
                }
            }

            best_artist.map(str::to_string)
        }
    }

    let mut groups: HashMap<String, AlbumGroup> = HashMap::new();

    for song in songs {
        let album_name =
            normalize_text(song.album.as_deref()).unwrap_or_else(|| UNKNOWN_ALBUM_NAME.to_string());
        let key = album_key(&song);
        let id = normalize_text(song.album_id.as_deref()).unwrap_or_else(|| key.clone());

        groups
            .entry(key)
            .or_insert_with(|| AlbumGroup::new(id, album_name))
            .absorb(&song);
    }

    let mut albums: Vec<_> = groups
        .into_values()
        .map(|group| {
            let artist = group.artist();

            FolderStructureAlbumItem {
                id: group.id,
                name: group.name,
                artist,
                cover_art_id: group.cover_art_id,
                year: group.year,
            }
        })
        .collect();

    albums.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| {
                left.artist
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .cmp(&right.artist.as_deref().unwrap_or("").to_ascii_lowercase())
            })
            .then_with(|| {
                left.year
                    .unwrap_or_default()
                    .cmp(&right.year.unwrap_or_default())
            })
    });

    albums
}

fn album_key(song: &SongRow) -> String {
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
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use opensubsonic_client::{ApiError, ResponseMeta, ResponseStatus};
    use serde_json::json;

    use super::{load_albums, AlbumsApi, ApiDirectoryResponse, GetMusicDirectoryRequest};
    use opensubsonic_client::Envelope;

    #[derive(Debug, Clone)]
    enum MockResponse {
        Ok(serde_json::Value),
    }

    #[derive(Debug, Clone, Default)]
    struct MockAlbumsApi {
        directory_responses: std::collections::HashMap<String, MockResponse>,
        call_log: Arc<Mutex<Vec<String>>>,
    }

    impl MockAlbumsApi {
        fn with_directory_response(mut self, id: &str, response: MockResponse) -> Self {
            self.directory_responses.insert(id.to_string(), response);
            self
        }

        fn calls(&self) -> Vec<String> {
            self.call_log.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl AlbumsApi for MockAlbumsApi {
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
        media_type: Option<&str>,
        year: Option<u32>,
        cover_art_id: Option<&str>,
    ) -> serde_json::Value {
        json!({
            "id": id,
            "isDir": false,
            "album": album,
            "albumId": album_id,
            "artist": artist,
            "coverArt": cover_art_id,
            "year": year,
            "mediaType": media_type
        })
    }

    #[tokio::test]
    async fn load_albums_merges_split_album_parts_into_one_card() {
        let api = MockAlbumsApi::default()
            .with_directory_response(
                "artist-a",
                MockResponse::Ok(json!({
                    "id": "artist-a",
                    "name": "Artist A",
                    "child": [
                        {
                            "id": "album-c-part-1",
                            "isDir": true
                        },
                        {
                            "id": "album-c-part-2",
                            "isDir": true
                        }
                    ]
                })),
            )
            .with_directory_response(
                "album-c-part-1",
                MockResponse::Ok(json!({
                    "id": "album-c-part-1",
                    "name": "AlbumC_parted",
                    "child": [
                        folder_song("song-1", Some("Album C"), Some("album-c"), Some("Artist A"), Some("song"), Some(2024), Some("cover-1"))
                    ]
                })),
            )
            .with_directory_response(
                "album-c-part-2",
                MockResponse::Ok(json!({
                    "id": "album-c-part-2",
                    "name": "AlbumC_parted2",
                    "child": [
                        folder_song("song-2", Some("Album C"), Some("album-c"), Some("Artist A"), Some("song"), Some(2024), Some("cover-2"))
                    ]
                })),
            );

        let response = load_albums(&api, "1", "artist-a").await.unwrap();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].id, "album-c");
        assert_eq!(response.albums[0].name, "Album C");
        assert_eq!(response.albums[0].artist.as_deref(), Some("Artist A"));
        assert_eq!(response.albums[0].cover_art_id.as_deref(), Some("cover-1"));
    }

    #[tokio::test]
    async fn load_albums_uses_song_artist_for_album_artist_label() {
        let api = MockAlbumsApi::default()
            .with_directory_response(
                "artist-a",
                MockResponse::Ok(json!({
                    "id": "artist-a",
                    "name": "ArtistA",
                    "child": [
                        {
                            "id": "album-b",
                            "isDir": true
                        }
                    ]
                })),
            )
            .with_directory_response(
                "album-b",
                MockResponse::Ok(json!({
                    "id": "album-b",
                    "name": "Album B",
                    "child": [
                        folder_song("song-1", Some("Album B"), Some("album-b-id"), Some("ArtistB"), Some("song"), Some(2023), Some("cover-b"))
                    ]
                })),
            );

        let response = load_albums(&api, "1", "artist-a").await.unwrap();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].artist.as_deref(), Some("ArtistB"));
    }

    #[tokio::test]
    async fn load_albums_uses_unknown_album_fallback() {
        let api = MockAlbumsApi::default().with_directory_response(
            "folder-root",
            MockResponse::Ok(json!({
                "id": "folder-root",
                "name": "Folder Root",
                "child": [
                    folder_song("song-1", None, None, Some("Loose Artist"), Some("song"), Some(2022), None)
                ]
            })),
        );

        let response = load_albums(&api, "1", "folder-root").await.unwrap();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].name, "Unknown Album");
        assert_eq!(response.albums[0].artist.as_deref(), Some("Loose Artist"));
    }

    #[tokio::test]
    async fn load_albums_does_not_visit_the_same_node_twice() {
        let api = MockAlbumsApi::default()
            .with_directory_response(
                "root",
                MockResponse::Ok(json!({
                    "id": "root",
                    "name": "Root",
                    "child": [
                        {
                            "id": "loop",
                            "isDir": true
                        },
                        {
                            "id": "loop",
                            "isDir": true
                        }
                    ]
                })),
            )
            .with_directory_response(
                "loop",
                MockResponse::Ok(json!({
                    "id": "loop",
                    "name": "Loop",
                    "child": [
                        {
                            "id": "root",
                            "isDir": true
                        },
                        folder_song("song-1", Some("Album Loop"), Some("album-loop"), Some("Artist Loop"), Some("song"), Some(2021), None)
                    ]
                })),
            );

        let response = load_albums(&api, "1", "root").await.unwrap();
        let calls = api.calls();
        let root_calls = calls
            .iter()
            .filter(|call| *call == "directory:root")
            .count();
        let loop_calls = calls
            .iter()
            .filter(|call| *call == "directory:loop")
            .count();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(root_calls, 1);
        assert_eq!(loop_calls, 1);
    }
}
