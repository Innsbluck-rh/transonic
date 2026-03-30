use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::models::{
    AlbumListItem, ArtistIndexItem, MusicDirectoryChild, MusicDirectoryResponse, MusicFolderSummary,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbumListPayload {
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    album: Vec<RawAlbumListItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAlbumListItem {
    id: String,
    name: String,
    artist: Option<String>,
    cover_art: Option<String>,
    year: Option<u32>,
}

impl From<RawAlbumListItem> for AlbumListItem {
    fn from(value: RawAlbumListItem) -> Self {
        Self {
            id: value.id,
            name: value.name,
            artist: value.artist,
            cover_art_id: value.cover_art,
            year: value.year,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawMusicFoldersPayload {
    #[serde(
        rename = "musicFolder",
        default,
        deserialize_with = "deserialize_vec_or_single"
    )]
    music_folder: Vec<RawMusicFolder>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMusicFolder {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    name: String,
}

impl From<RawMusicFolder> for MusicFolderSummary {
    fn from(value: RawMusicFolder) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndexesPayload {
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    index: Vec<RawIndex>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndex {
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    artist: Vec<RawIndexArtist>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawIndexArtist {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    name: String,
}

impl From<RawIndexArtist> for ArtistIndexItem {
    fn from(value: RawIndexArtist) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawDirectoryPayload {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    name: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_single")]
    child: Vec<RawDirectoryChild>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDirectoryChild {
    #[serde(deserialize_with = "deserialize_stringish")]
    id: String,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    parent: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    path: Option<String>,
    title: Option<String>,
    name: Option<String>,
    artist: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    artist_id: Option<String>,
    album: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    album_id: Option<String>,
    cover_art: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    media_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    content_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_stringish")]
    item_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    is_dir: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_boolish")]
    is_video: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_u32ish")]
    track: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_optional_u32ish")]
    disc_number: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_optional_u32ish")]
    year: Option<u32>,
}

impl From<RawDirectoryChild> for MusicDirectoryChild {
    fn from(value: RawDirectoryChild) -> Self {
        let is_directory = value.is_dir.unwrap_or(false);
        let album = value.album.clone();

        Self {
            id: value.id,
            parent_id: value.parent,
            path: value.path,
            title: value
                .title
                .or(value.name)
                .or(album.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            album,
            album_id: value.album_id,
            artist: value.artist,
            artist_id: value.artist_id,
            cover_art_id: value.cover_art,
            track: value.track,
            disc_number: value.disc_number,
            year: value.year,
            is_directory,
            media_type: normalize_media_type(
                value.media_type,
                value.item_type,
                value.content_type,
                value.is_video,
                is_directory,
            ),
        }
    }
}

pub(crate) fn parse_album_list_items(payload: Value) -> Result<Vec<AlbumListItem>, String> {
    let payload: RawAlbumListPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the album list payload: {error}"))?;

    Ok(payload.album.into_iter().map(AlbumListItem::from).collect())
}

pub(crate) fn parse_music_folders(payload: Value) -> Result<Vec<MusicFolderSummary>, String> {
    let payload: RawMusicFoldersPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the music folders payload: {error}"))?;

    Ok(payload
        .music_folder
        .into_iter()
        .map(MusicFolderSummary::from)
        .collect())
}

pub(crate) fn parse_artist_indexes(payload: Value) -> Result<Vec<ArtistIndexItem>, String> {
    let payload: RawIndexesPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the indexes payload: {error}"))?;

    Ok(payload
        .index
        .into_iter()
        .flat_map(|index| index.artist)
        .map(ArtistIndexItem::from)
        .collect())
}

pub(crate) fn parse_music_directory(payload: Value) -> Result<MusicDirectoryResponse, String> {
    let payload: RawDirectoryPayload = parse_value(payload)
        .map_err(|error| format!("Failed to parse the music directory payload: {error}"))?;

    Ok(MusicDirectoryResponse {
        id: payload.id,
        name: payload.name,
        children: payload
            .child
            .into_iter()
            .map(MusicDirectoryChild::from)
            .collect(),
    })
}

fn deserialize_vec_or_single<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| parse_value(item).map_err(serde::de::Error::custom))
            .collect(),
        Value::Object(_) => Ok(vec![parse_value(value).map_err(serde::de::Error::custom)?]),
        Value::Null => Ok(Vec::new()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected collection payload: {other}"
        ))),
    }
}

fn deserialize_stringish<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::String(value) => Ok(value),
        Value::Number(value) => Ok(value.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected string payload: {other}"
        ))),
    }
}

fn deserialize_optional_stringish<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        Value::String(value) => Ok(Some(value)),
        Value::Number(value) => Ok(Some(value.to_string())),
        other => Err(serde::de::Error::custom(format!(
            "unexpected string payload: {other}"
        ))),
    }
}

fn deserialize_optional_boolish<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        Value::Bool(value) => Ok(Some(value)),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Ok(Some(true)),
            "false" | "0" => Ok(Some(false)),
            other => Err(serde::de::Error::custom(format!(
                "unexpected bool payload: {other}"
            ))),
        },
        Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                match value {
                    0 => Ok(Some(false)),
                    1 => Ok(Some(true)),
                    other => Err(serde::de::Error::custom(format!(
                        "unexpected bool payload: {other}"
                    ))),
                }
            } else {
                Err(serde::de::Error::custom(format!(
                    "unexpected bool payload: {value}"
                )))
            }
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected bool payload: {other}"
        ))),
    }
}

fn deserialize_optional_u32ish<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        Value::Number(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("unexpected year payload: {value}"))),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            trimmed.parse::<u32>().map(Some).map_err(|error| {
                serde::de::Error::custom(format!("unexpected year payload: {error}"))
            })
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected year payload: {other}"
        ))),
    }
}

fn normalize_media_type(
    media_type: Option<String>,
    item_type: Option<String>,
    content_type: Option<String>,
    is_video: Option<bool>,
    is_directory: bool,
) -> Option<String> {
    if is_directory {
        return None;
    }

    if let Some(media_type) = normalize_text(media_type.as_deref()) {
        return Some(media_type.to_ascii_lowercase());
    }

    if is_video.unwrap_or(false) {
        return Some("video".to_string());
    }

    if let Some(content_type) = normalize_text(content_type.as_deref()) {
        if content_type.to_ascii_lowercase().starts_with("audio/") {
            return Some("song".to_string());
        }
    }

    if let Some(item_type) = normalize_text(item_type.as_deref()) {
        if item_type.eq_ignore_ascii_case("music") {
            return Some("song".to_string());
        }
    }

    Some("song".to_string())
}

fn normalize_text(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn parse_value<T>(value: Value) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        parse_album_list_items, parse_artist_indexes, parse_music_directory, parse_music_folders,
    };

    #[test]
    fn parse_album_list_items_accepts_array_payloads() {
        let albums = parse_album_list_items(json!({
            "album": [
                {
                    "id": "album-1",
                    "name": "First Album",
                    "artist": "First Artist",
                    "coverArt": "cover-1",
                    "year": 2024
                }
            ]
        }))
        .unwrap();

        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].id, "album-1");
        assert_eq!(albums[0].name, "First Album");
        assert_eq!(albums[0].artist.as_deref(), Some("First Artist"));
        assert_eq!(albums[0].cover_art_id.as_deref(), Some("cover-1"));
        assert_eq!(albums[0].year, Some(2024));
    }

    #[test]
    fn parse_album_list_items_accepts_single_album_objects() {
        let albums = parse_album_list_items(json!({
            "album": {
                "id": "album-1",
                "name": "First Album"
            }
        }))
        .unwrap();

        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].name, "First Album");
        assert_eq!(albums[0].artist, None);
    }

    #[test]
    fn parse_music_folders_accepts_single_folder_objects() {
        let folders = parse_music_folders(json!({
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
    fn parse_music_folders_accepts_array_payloads() {
        let folders = parse_music_folders(json!({
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

    #[test]
    fn parse_artist_indexes_accepts_single_artist_objects() {
        let artists = parse_artist_indexes(json!({
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
    fn parse_artist_indexes_flattens_multiple_index_groups() {
        let artists = parse_artist_indexes(json!({
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

    #[test]
    fn parse_music_directory_keeps_directory_and_file_children() {
        let response = parse_music_directory(json!({
            "id": "artist-1",
            "name": "Artist One",
            "child": [
                {
                    "id": "album-1",
                    "isDir": true,
                    "title": "Album One",
                    "artist": "Artist One",
                    "coverArt": "cover-1",
                    "year": "2024"
                },
                {
                    "id": "song-1",
                    "isDir": false,
                    "title": "Song One",
                    "artist": "Artist One"
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.id, "artist-1");
        assert_eq!(response.name, "Artist One");
        assert_eq!(response.children.len(), 2);
        assert_eq!(response.children[0].title, "Album One");
        assert_eq!(response.children[0].parent_id.as_deref(), None);
        assert_eq!(response.children[0].album, None);
        assert_eq!(response.children[0].media_type, None);
        assert_eq!(
            response.children[0].cover_art_id.as_deref(),
            Some("cover-1")
        );
        assert_eq!(response.children[0].year, Some(2024));
        assert!(response.children[0].is_directory);
        assert_eq!(response.children[1].title, "Song One");
        assert_eq!(response.children[1].media_type.as_deref(), Some("song"));
        assert!(!response.children[1].is_directory);
    }
}
