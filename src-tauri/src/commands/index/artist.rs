use opensubsonic_client::api::browsing::{BrowsingApi, GetIndexesRequest};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    commands::{
        common::{client, format_api_error, normalize_optional_text, normalize_roles, trim_text},
        json::{opt_stringish, opt_u32ish, stringish, value_as, vec_or_single},
    },
    models::{ArtistGroup, ArtistSummary, ArtistsRequest, ArtistsResponse},
    ActiveSessionState,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIndexes {
    #[serde(default, deserialize_with = "opt_stringish")]
    ignored_articles: Option<String>,
    #[serde(default, deserialize_with = "vec_or_single")]
    index: Vec<RawIndex>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndex {
    name: String,
    #[serde(default, deserialize_with = "vec_or_single")]
    artist: Vec<RawArtist>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawArtist {
    #[serde(deserialize_with = "stringish")]
    id: String,
    name: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    album_count: Option<u32>,
    #[serde(default, deserialize_with = "opt_stringish")]
    artist_image_url: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    starred: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    music_brainz_id: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    sort_name: Option<String>,
    #[serde(default)]
    roles: Vec<String>,
}

impl From<RawArtist> for ArtistSummary {
    fn from(value: RawArtist) -> Self {
        Self {
            id: value.id,
            name: value.name,
            cover_art_id: normalize_optional_text(value.cover_art),
            album_count: value.album_count,
            artist_image_url: normalize_optional_text(value.artist_image_url),
            starred: normalize_optional_text(value.starred),
            music_brainz_id: normalize_optional_text(value.music_brainz_id),
            sort_name: normalize_optional_text(value.sort_name),
            roles: normalize_roles(value.roles),
        }
    }
}

impl From<RawIndex> for ArtistGroup {
    fn from(value: RawIndex) -> Self {
        Self {
            name: value.name,
            artists: value.artist.into_iter().map(ArtistSummary::from).collect(),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_artist_indexes(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: Option<ArtistsRequest>,
) -> Result<ArtistsResponse, String> {
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

    parse_indexes(response.payload.indexes)
}

fn parse_indexes(payload: Value) -> Result<ArtistsResponse, String> {
    let payload: RawIndexes = value_as(payload)
        .map_err(|error| format!("Failed to parse the indexes payload: {error}"))?;

    Ok(ArtistsResponse {
        ignored_articles: normalize_optional_text(payload.ignored_articles),
        indexes: payload.index.into_iter().map(ArtistGroup::from).collect(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_indexes;

    #[test]
    fn parse_indexes_preserves_index_grouping() {
        let response = parse_indexes(json!({
            "ignoredArticles": "The An A",
            "index": {
                "name": "A",
                "artist": {
                    "id": "artist-1",
                    "name": "Artist One",
                    "coverArt": "ar-1",
                    "albumCount": "3",
                    "roles": ["artist"]
                }
            }
        }))
        .unwrap();

        assert_eq!(response.ignored_articles.as_deref(), Some("The An A"));
        assert_eq!(response.indexes.len(), 1);
        assert_eq!(response.indexes[0].name, "A");
        assert_eq!(response.indexes[0].artists[0].id, "artist-1");
        assert_eq!(response.indexes[0].artists[0].name, "Artist One");
        assert_eq!(
            response.indexes[0].artists[0].cover_art_id.as_deref(),
            Some("ar-1")
        );
        assert_eq!(response.indexes[0].artists[0].album_count, Some(3));
        assert_eq!(response.indexes[0].artists[0].roles, vec!["artist"]);
    }

    #[test]
    fn parse_indexes_handles_multiple_index_groups() {
        let response = parse_indexes(json!({
            "index": [
                {
                    "name": "A",
                    "artist": [
                        { "id": "artist-1", "name": "Artist One" }
                    ]
                },
                {
                    "name": "B",
                    "artist": {
                        "id": "artist-2",
                        "name": "Artist Two"
                    }
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.indexes.len(), 2);
        assert_eq!(response.indexes[0].name, "A");
        assert_eq!(response.indexes[1].name, "B");
        assert_eq!(response.indexes[1].artists[0].id, "artist-2");
    }
}
