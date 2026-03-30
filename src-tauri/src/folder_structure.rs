use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use opensubsonic_client::{
    api::browsing::{
        BrowsingApi, GetIndexesRequest, GetIndexesResponse, GetMusicDirectoryRequest,
        GetMusicDirectoryResponse as ApiGetMusicDirectoryResponse,
    },
    ApiError, Envelope,
};

use crate::{
    browse_parser::{parse_artist_indexes, parse_music_directory},
    models::{
        FolderStructureAlbumItem, FolderStructureAlbumsResponse, FolderStructureRootNode,
        FolderStructureRootsResponse, FolderStructureSource, MusicDirectoryChild,
        MusicFolderSummary,
    },
};

const UNKNOWN_ALBUM_NAME: &str = "Unknown Album";

#[async_trait]
pub(crate) trait FolderStructureBrowsingApi: Send + Sync {
    async fn get_indexes(
        &self,
        req: GetIndexesRequest,
    ) -> Result<Envelope<GetIndexesResponse>, ApiError>;

    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<ApiGetMusicDirectoryResponse>, ApiError>;
}

#[async_trait]
impl<T> FolderStructureBrowsingApi for T
where
    T: BrowsingApi + Send + Sync,
{
    async fn get_indexes(
        &self,
        req: GetIndexesRequest,
    ) -> Result<Envelope<GetIndexesResponse>, ApiError> {
        BrowsingApi::get_indexes(self, req).await
    }

    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<ApiGetMusicDirectoryResponse>, ApiError> {
        BrowsingApi::get_music_directory(self, req).await
    }
}

pub(crate) async fn load_folder_structure_roots<C>(
    client: &C,
    library: &MusicFolderSummary,
) -> Result<FolderStructureRootsResponse, ApiError>
where
    C: FolderStructureBrowsingApi,
{
    match client
        .get_music_directory(GetMusicDirectoryRequest {
            id: library.id.clone(),
        })
        .await
    {
        Ok(response) => {
            let directory =
                parse_music_directory(response.payload.directory).map_err(ApiError::Protocol)?;

            Ok(FolderStructureRootsResponse {
                library_id: library.id.clone(),
                library_name: library.name.clone(),
                source: FolderStructureSource::Directory,
                root_nodes: root_nodes_from_directory_children(&directory.children),
            })
        }
        Err(error) if should_use_indexes_fallback(&error) => {
            let response = client
                .get_indexes(GetIndexesRequest {
                    music_folder_id: Some(library.id.clone()),
                    if_modified_since: None,
                })
                .await?;
            let artists =
                parse_artist_indexes(response.payload.indexes).map_err(ApiError::Protocol)?;

            Ok(FolderStructureRootsResponse {
                library_id: library.id.clone(),
                library_name: library.name.clone(),
                source: FolderStructureSource::Indexes,
                root_nodes: artists
                    .into_iter()
                    .map(|artist| FolderStructureRootNode {
                        id: artist.id,
                        name: artist.name,
                    })
                    .collect(),
            })
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn load_folder_structure_albums<C>(
    client: &C,
    library_id: &str,
    node_id: &str,
) -> Result<FolderStructureAlbumsResponse, ApiError>
where
    C: FolderStructureBrowsingApi,
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
        let directory =
            parse_music_directory(response.payload.directory).map_err(ApiError::Protocol)?;

        if node_name.is_none() {
            node_name = Some(directory.name.clone());
        }

        let mut child_directories = Vec::new();
        for child in directory.children {
            if child.is_directory {
                child_directories.push(child.id.clone());
                continue;
            }

            if child.media_type.as_deref() == Some("song") {
                songs.push(child);
            }
        }

        for child_directory_id in child_directories.into_iter().rev() {
            stack.push(child_directory_id);
        }
    }

    Ok(FolderStructureAlbumsResponse {
        library_id: library_id.to_string(),
        node_id: node_id.to_string(),
        node_name: node_name.unwrap_or_else(|| "[Unknown]".to_string()),
        albums: aggregate_folder_structure_albums(songs),
    })
}

fn root_nodes_from_directory_children(
    children: &[MusicDirectoryChild],
) -> Vec<FolderStructureRootNode> {
    children
        .iter()
        .filter(|child| child.is_directory)
        .map(|child| FolderStructureRootNode {
            id: child.id.clone(),
            name: child.title.clone(),
        })
        .collect()
}

fn should_use_indexes_fallback(error: &ApiError) -> bool {
    match error {
        ApiError::Api { code, .. } if *code == 70 => true,
        ApiError::Api { message, .. } => message
            .as_deref()
            .map(|message| message.to_ascii_lowercase().contains("directory not found"))
            .unwrap_or(false),
        _ => false,
    }
}

fn aggregate_folder_structure_albums(
    songs: Vec<MusicDirectoryChild>,
) -> Vec<FolderStructureAlbumItem> {
    #[derive(Debug)]
    struct AlbumAccumulator {
        id: String,
        name: String,
        artist_counts: HashMap<String, usize>,
        artist_order: Vec<String>,
        cover_art_id: Option<String>,
        year: Option<u32>,
    }

    impl AlbumAccumulator {
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

        fn absorb_song(&mut self, song: &MusicDirectoryChild) {
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

        fn representative_artist(&self) -> Option<String> {
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

    let mut groups: HashMap<String, AlbumAccumulator> = HashMap::new();

    for song in songs {
        let album_name =
            normalize_text(song.album.as_deref()).unwrap_or_else(|| UNKNOWN_ALBUM_NAME.to_string());
        let group_key = album_group_key(&song);
        let group_id =
            normalize_text(song.album_id.as_deref()).unwrap_or_else(|| group_key.clone());

        groups
            .entry(group_key)
            .or_insert_with(|| AlbumAccumulator::new(group_id, album_name))
            .absorb_song(&song);
    }

    let mut albums: Vec<_> = groups
        .into_values()
        .map(|group| {
            let artist = group.representative_artist();

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

fn album_group_key(song: &MusicDirectoryChild) -> String {
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

fn normalize_text(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use opensubsonic_client::{ResponseMeta, ResponseStatus};
    use serde_json::json;

    use super::{
        load_folder_structure_albums, load_folder_structure_roots, ApiGetMusicDirectoryResponse,
        FolderStructureBrowsingApi, GetIndexesRequest, GetIndexesResponse,
        GetMusicDirectoryRequest, MusicFolderSummary,
    };
    use crate::models::FolderStructureSource;
    use opensubsonic_client::{ApiError, Envelope};

    #[derive(Debug, Clone)]
    enum MockResponse {
        Ok(serde_json::Value),
        ApiError { code: u32, message: Option<String> },
    }

    #[derive(Debug, Clone, Default)]
    struct MockFolderStructureApi {
        directory_responses: std::collections::HashMap<String, MockResponse>,
        indexes_responses: std::collections::HashMap<Option<String>, MockResponse>,
        call_log: Arc<Mutex<Vec<String>>>,
    }

    impl MockFolderStructureApi {
        fn with_directory_response(mut self, id: &str, response: MockResponse) -> Self {
            self.directory_responses.insert(id.to_string(), response);
            self
        }

        fn with_indexes_response(
            mut self,
            music_folder_id: Option<&str>,
            response: MockResponse,
        ) -> Self {
            self.indexes_responses
                .insert(music_folder_id.map(str::to_string), response);
            self
        }

        fn calls(&self) -> Vec<String> {
            self.call_log.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl FolderStructureBrowsingApi for MockFolderStructureApi {
        async fn get_indexes(
            &self,
            req: GetIndexesRequest,
        ) -> Result<Envelope<GetIndexesResponse>, ApiError> {
            self.call_log
                .lock()
                .unwrap()
                .push(format!("indexes:{:?}", req.music_folder_id));

            match self
                .indexes_responses
                .get(&req.music_folder_id)
                .cloned()
                .expect("missing mock indexes response")
            {
                MockResponse::Ok(indexes) => Ok(ok_envelope(GetIndexesResponse { indexes })),
                MockResponse::ApiError { code, message } => Err(api_error(code, message)),
            }
        }

        async fn get_music_directory(
            &self,
            req: GetMusicDirectoryRequest,
        ) -> Result<Envelope<ApiGetMusicDirectoryResponse>, ApiError> {
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
                MockResponse::Ok(directory) => {
                    Ok(ok_envelope(ApiGetMusicDirectoryResponse { directory }))
                }
                MockResponse::ApiError { code, message } => Err(api_error(code, message)),
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

    fn api_error(code: u32, message: Option<String>) -> ApiError {
        ApiError::Api {
            code,
            message,
            help_url: None,
            meta: ResponseMeta {
                status: ResponseStatus::Failed,
                api_version: "1.16.1".to_string(),
                server_type: Some("mock".to_string()),
                server_version: Some("1.0.0".to_string()),
                open_subsonic: Some(true),
            },
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
            "parent": "album-node",
            "isDir": false,
            "title": format!("Song {id}"),
            "album": album,
            "albumId": album_id,
            "artist": artist,
            "coverArt": cover_art_id,
            "year": year,
            "track": 1,
            "discNumber": 1,
            "path": format!("root/{id}.flac"),
            "mediaType": media_type
        })
    }

    #[tokio::test]
    async fn load_folder_structure_roots_uses_music_directory_when_available() {
        let library = MusicFolderSummary {
            id: "1".to_string(),
            name: "Library".to_string(),
        };
        let api = MockFolderStructureApi::default().with_directory_response(
            "1",
            MockResponse::Ok(json!({
                "id": "1",
                "name": "Library",
                "child": [
                    {
                        "id": "artist-a",
                        "title": "Artist A",
                        "isDir": true
                    },
                    {
                        "id": "song-1",
                        "title": "Loose Song",
                        "isDir": false,
                        "mediaType": "song"
                    }
                ]
            })),
        );

        let response = load_folder_structure_roots(&api, &library).await.unwrap();

        assert_eq!(response.library_id, "1");
        assert_eq!(response.library_name, "Library");
        assert_eq!(response.source, FolderStructureSource::Directory);
        assert_eq!(response.root_nodes.len(), 1);
        assert_eq!(response.root_nodes[0].id, "artist-a");
        assert_eq!(response.root_nodes[0].name, "Artist A");
        assert_eq!(api.calls(), vec!["directory:1"]);
    }

    #[tokio::test]
    async fn load_folder_structure_roots_falls_back_to_indexes_for_directory_not_found() {
        let library = MusicFolderSummary {
            id: "1".to_string(),
            name: "Library".to_string(),
        };
        let api = MockFolderStructureApi::default()
            .with_directory_response(
                "1",
                MockResponse::ApiError {
                    code: 70,
                    message: Some("Directory not found".to_string()),
                },
            )
            .with_indexes_response(
                Some("1"),
                MockResponse::Ok(json!({
                    "index": [
                        {
                            "artist": [
                                {
                                    "id": "artist-a",
                                    "name": "Artist A"
                                }
                            ]
                        }
                    ]
                })),
            );

        let response = load_folder_structure_roots(&api, &library).await.unwrap();

        assert_eq!(response.source, FolderStructureSource::Indexes);
        assert_eq!(response.root_nodes.len(), 1);
        assert_eq!(response.root_nodes[0].id, "artist-a");
        assert_eq!(api.calls(), vec!["directory:1", "indexes:Some(\"1\")"]);
    }

    #[tokio::test]
    async fn load_folder_structure_albums_merges_split_album_parts_into_one_card() {
        let api = MockFolderStructureApi::default()
            .with_directory_response(
                "artist-a",
                MockResponse::Ok(json!({
                    "id": "artist-a",
                    "name": "Artist A",
                    "child": [
                        {
                            "id": "album-c-part-1",
                            "title": "AlbumC_parted",
                            "isDir": true
                        },
                        {
                            "id": "album-c-part-2",
                            "title": "AlbumC_parted2",
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

        let response = load_folder_structure_albums(&api, "1", "artist-a")
            .await
            .unwrap();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].id, "album-c");
        assert_eq!(response.albums[0].name, "Album C");
        assert_eq!(response.albums[0].artist.as_deref(), Some("Artist A"));
        assert_eq!(response.albums[0].cover_art_id.as_deref(), Some("cover-1"));
    }

    #[tokio::test]
    async fn load_folder_structure_albums_uses_song_artist_for_album_artist_label() {
        let api = MockFolderStructureApi::default()
            .with_directory_response(
                "artist-a",
                MockResponse::Ok(json!({
                    "id": "artist-a",
                    "name": "ArtistA",
                    "child": [
                        {
                            "id": "album-b",
                            "title": "Album B",
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

        let response = load_folder_structure_albums(&api, "1", "artist-a")
            .await
            .unwrap();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].artist.as_deref(), Some("ArtistB"));
    }

    #[tokio::test]
    async fn load_folder_structure_albums_uses_unknown_album_fallback() {
        let api = MockFolderStructureApi::default().with_directory_response(
            "folder-root",
            MockResponse::Ok(json!({
                "id": "folder-root",
                "name": "Folder Root",
                "child": [
                    folder_song("song-1", None, None, Some("Loose Artist"), Some("song"), Some(2022), None)
                ]
            })),
        );

        let response = load_folder_structure_albums(&api, "1", "folder-root")
            .await
            .unwrap();

        assert_eq!(response.albums.len(), 1);
        assert_eq!(response.albums[0].name, "Unknown Album");
        assert_eq!(response.albums[0].artist.as_deref(), Some("Loose Artist"));
    }

    #[tokio::test]
    async fn load_folder_structure_albums_does_not_visit_the_same_node_twice() {
        let api = MockFolderStructureApi::default()
            .with_directory_response(
                "root",
                MockResponse::Ok(json!({
                    "id": "root",
                    "name": "Root",
                    "child": [
                        {
                            "id": "loop",
                            "title": "Loop",
                            "isDir": true
                        },
                        {
                            "id": "loop",
                            "title": "Loop Duplicate",
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
                            "title": "Back To Root",
                            "isDir": true
                        },
                        folder_song("song-1", Some("Album Loop"), Some("album-loop"), Some("Artist Loop"), Some("song"), Some(2021), None)
                    ]
                })),
            );

        let response = load_folder_structure_albums(&api, "1", "root")
            .await
            .unwrap();
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
