use opensubsonic_client::api::browsing::{BrowsingApi, GetIndexesRequest};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error},
        json::{stringish, value_as, vec_or_single},
    },
    models::{ArtistIndexItem, ArtistIndexesRequest, ArtistIndexesResponse},
    ActiveSessionState,
};

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

impl From<RawArtist> for ArtistIndexItem {
    fn from(value: RawArtist) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_artist_indexes(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: Option<ArtistIndexesRequest>,
) -> Result<ArtistIndexesResponse, String> {
    let music_folder_id = payload
        .and_then(|payload| payload.music_folder_id)
        .and_then(|value| trim_text(&value));
    let client = client(&app, &state.0)?;
    let response = BrowsingApi::get_indexes(
        &client,
        GetIndexesRequest {
            music_folder_id,
            if_modified_since: None,
        },
    )
    .await
    .map_err(format_api_error)?;

    Ok(ArtistIndexesResponse {
        artists: parse_indexes(response.payload.indexes)?,
    })
}

fn parse_indexes(payload: Value) -> Result<Vec<ArtistIndexItem>, String> {
    let payload: RawIndexes = value_as(payload)
        .map_err(|error| format!("Failed to parse the indexes payload: {error}"))?;

    Ok(payload
        .index
        .into_iter()
        .flat_map(|group| group.artist)
        .map(ArtistIndexItem::from)
        .collect())
}

fn trim_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_indexes;

    #[test]
    fn parse_indexes_accepts_single_artist_objects() {
        let artists = parse_indexes(json!({
            "index": {
                "artist": {
                    "id": "artist-1",
                    "name": "Artist One"
                }
            }
        }))
        .unwrap();

        assert_eq!(artists.len(), 1);
        assert_eq!(artists[0].id, "artist-1");
        assert_eq!(artists[0].name, "Artist One");
    }

    #[test]
    fn parse_indexes_flattens_multiple_index_groups() {
        let artists = parse_indexes(json!({
            "index": [
                {
                    "artist": [
                        {
                            "id": "artist-1",
                            "name": "Artist One"
                        }
                    ]
                },
                {
                    "artist": {
                        "id": "artist-2",
                        "name": "Artist Two"
                    }
                }
            ]
        }))
        .unwrap();

        assert_eq!(artists.len(), 2);
        assert_eq!(artists[1].id, "artist-2");
        assert_eq!(artists[1].name, "Artist Two");
    }
}
