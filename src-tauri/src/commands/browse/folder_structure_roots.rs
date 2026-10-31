use async_trait::async_trait;
use opensubsonic_client::{
    api::browsing::{
        BrowsingApi, GetIndexesRequest, GetIndexesResponse, GetMusicDirectoryRequest,
        GetMusicDirectoryResponse as ApiDirectoryResponse, GetMusicFoldersRequest,
    },
    ApiError, Envelope,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{opt_boolish, stringish, value_as, vec_or_single},
    },
    models::{
        FolderStructureRootNode, FolderStructureRootsRequest, FolderStructureRootsResponse,
        FolderStructureSource, MusicFolderSummary, MusicFoldersResponse,
    },
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
struct RawFolders {
    #[serde(rename = "musicFolder", default, deserialize_with = "vec_or_single")]
    music_folder: Vec<RawFolder>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFolder {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: String,
}

impl From<RawFolder> for MusicFolderSummary {
    fn from(value: RawFolder) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndexes {
    #[serde(default, deserialize_with = "vec_or_single")]
    index: Vec<RawIndex>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndex {
    #[serde(default, deserialize_with = "vec_or_single")]
    artist: Vec<RawArtist>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawArtist {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: String,
}

impl From<RawArtist> for FolderStructureRootNode {
    fn from(value: RawArtist) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

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
    title: String,
    #[serde(default, deserialize_with = "opt_boolish")]
    is_dir: Option<bool>,
}

impl RawChild {
    fn is_directory(&self) -> bool {
        self.is_dir.unwrap_or(false)
    }
}

#[async_trait]
trait RootsApi: Send + Sync {
    async fn get_indexes(
        &self,
        req: GetIndexesRequest,
    ) -> Result<Envelope<GetIndexesResponse>, ApiError>;

    async fn get_music_directory(
        &self,
        req: GetMusicDirectoryRequest,
    ) -> Result<Envelope<ApiDirectoryResponse>, ApiError>;
}

#[async_trait]
impl<T> RootsApi for T
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
    ) -> Result<Envelope<ApiDirectoryResponse>, ApiError> {
        BrowsingApi::get_music_directory(self, req).await
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_music_folders(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
) -> Result<MusicFoldersResponse, String> {
    let client = client(&app, &state.0)?;
    let response = client
        .get_music_folders(GetMusicFoldersRequest::default())
        .await
        .map_err(format_api_error)?;

    Ok(MusicFoldersResponse {
        music_folders: parse_folders(response.payload.music_folders)?,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_folder_structure_roots(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: FolderStructureRootsRequest,
) -> Result<FolderStructureRootsResponse, String> {
    let library_id = payload.library_id.trim();
    if library_id.is_empty() {
        return Err("libraryId is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    let folders = client
        .get_music_folders(GetMusicFoldersRequest::default())
        .await
        .map_err(format_api_error)?;
    let library = parse_folders(folders.payload.music_folders)?
        .into_iter()
        .find(|folder| folder.id == library_id)
        .ok_or_else(|| format!("Music folder not found: {library_id}"))?;

    load_roots(&client, &library)
        .await
        .map_err(format_api_error)
}

fn parse_folders(payload: Value) -> Result<Vec<MusicFolderSummary>, String> {
    let payload: RawFolders = value_as(payload)
        .map_err(|error| format!("Failed to parse the music folders payload: {error}"))?;

    Ok(payload
        .music_folder
        .into_iter()
        .map(MusicFolderSummary::from)
        .collect())
}

fn parse_root_indexes(payload: Value) -> Result<Vec<FolderStructureRootNode>, String> {
    let payload: RawIndexes = value_as(payload)
        .map_err(|error| format!("Failed to parse the indexes payload: {error}"))?;

    Ok(payload
        .index
        .into_iter()
        .flat_map(|group| group.artist)
        .map(FolderStructureRootNode::from)
        .collect())
}

fn parse_root_directory(payload: Value) -> Result<Vec<FolderStructureRootNode>, String> {
    let payload: RawDirectory = value_as(payload)
        .map_err(|error| format!("Failed to parse the music directory payload: {error}"))?;

    Ok(payload
        .child
        .into_iter()
        .filter(RawChild::is_directory)
        .map(|child| FolderStructureRootNode {
            id: child.id,
            name: child.title,
        })
        .collect())
}

async fn load_roots<C>(
    client: &C,
    library: &MusicFolderSummary,
) -> Result<FolderStructureRootsResponse, ApiError>
where
    C: RootsApi,
{
    match client
        .get_music_directory(GetMusicDirectoryRequest {
            id: library.id.clone(),
        })
        .await
    {
        Ok(response) => Ok(FolderStructureRootsResponse {
            library_id: library.id.clone(),
            library_name: library.name.clone(),
            source: FolderStructureSource::Directory,
            root_nodes: parse_root_directory(response.payload.directory)
                .map_err(ApiError::Protocol)?,
        }),
        Err(error) if should_use_indexes_fallback(&error) => {
            let response = client
                .get_indexes(GetIndexesRequest {
                    music_folder_id: Some(library.id.clone()),
                    if_modified_since: None,
                })
                .await?;

            Ok(FolderStructureRootsResponse {
                library_id: library.id.clone(),
                library_name: library.name.clone(),
                source: FolderStructureSource::Indexes,
                root_nodes: parse_root_indexes(response.payload.indexes)
                    .map_err(ApiError::Protocol)?,
            })
        }
        Err(error) => Err(error),
    }
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use opensubsonic_client::{ApiError, ResponseMeta, ResponseStatus};
    use serde_json::json;

    use super::{
        load_roots, parse_folders, ApiDirectoryResponse, GetIndexesRequest, GetIndexesResponse,
        GetMusicDirectoryRequest, MusicFolderSummary, RootsApi,
    };
    use crate::models::FolderStructureSource;
    use opensubsonic_client::Envelope;

    #[derive(Debug, Clone)]
    enum MockResponse {
        Ok(serde_json::Value),
        ApiError { code: u32, message: Option<String> },
    }

    #[derive(Debug, Clone, Default)]
    struct MockRootsApi {
        directory_responses: std::collections::HashMap<String, MockResponse>,
        indexes_responses: std::collections::HashMap<Option<String>, MockResponse>,
        call_log: Arc<Mutex<Vec<String>>>,
    }

    impl MockRootsApi {
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
    impl RootsApi for MockRootsApi {
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

    #[test]
    fn parse_folders_accepts_single_folder_objects() {
        let folders = parse_folders(json!({
            "musicFolder": {
                "id": 1,
                "name": "music"
            }
        }))
        .unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].id, "1");
        assert_eq!(folders[0].name, "music");
    }

    #[test]
    fn parse_folders_accepts_array_payloads() {
        let folders = parse_folders(json!({
            "musicFolder": [
                {
                    "id": 1,
                    "name": "music"
                },
                {
                    "id": "4",
                    "name": "upload"
                }
            ]
        }))
        .unwrap();

        assert_eq!(folders.len(), 2);
        assert_eq!(folders[1].id, "4");
        assert_eq!(folders[1].name, "upload");
    }

    #[tokio::test]
    async fn load_roots_uses_music_directory_when_available() {
        let library = MusicFolderSummary {
            id: "1".to_string(),
            name: "Library".to_string(),
        };
        let api = MockRootsApi::default().with_directory_response(
            "1",
            MockResponse::Ok(json!({
                "child": [
                    {
                        "id": "artist-a",
                        "title": "Artist A",
                        "isDir": true
                    },
                    {
                        "id": "song-1",
                        "title": "Loose Song",
                        "isDir": false
                    }
                ]
            })),
        );

        let response = load_roots(&api, &library).await.unwrap();

        assert_eq!(response.library_id, "1");
        assert_eq!(response.library_name, "Library");
        assert_eq!(response.source, FolderStructureSource::Directory);
        assert_eq!(response.root_nodes.len(), 1);
        assert_eq!(response.root_nodes[0].id, "artist-a");
        assert_eq!(response.root_nodes[0].name, "Artist A");
        assert_eq!(api.calls(), vec!["directory:1"]);
    }

    #[tokio::test]
    async fn load_roots_falls_back_to_indexes_for_directory_not_found() {
        let library = MusicFolderSummary {
            id: "1".to_string(),
            name: "Library".to_string(),
        };
        let api = MockRootsApi::default()
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

        let response = load_roots(&api, &library).await.unwrap();

        assert_eq!(response.source, FolderStructureSource::Indexes);
        assert_eq!(response.root_nodes.len(), 1);
        assert_eq!(response.root_nodes[0].id, "artist-a");
        assert_eq!(api.calls(), vec!["directory:1", "indexes:Some(\"1\")"]);
    }
}
