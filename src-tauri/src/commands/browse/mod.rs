pub mod album;
pub mod artist;
pub mod folder_structure;
pub mod folder_structure_album_songs;
pub mod song;

use serde::Deserialize;

use crate::commands::common::normalize_media_type;
use crate::commands::json::{opt_boolish, opt_stringish, opt_u32ish, opt_u64ish, stringish};
use crate::models::SongResponse;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawSong {
    #[serde(deserialize_with = "stringish")]
    pub id: String,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub parent: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub path: Option<String>,
    pub title: String,
    pub album: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub album_id: Option<String>,
    pub artist: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub artist_id: Option<String>,
    pub cover_art: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub track: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub disc_number: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub year: Option<u32>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub duration: Option<u32>,
    #[serde(default, deserialize_with = "opt_u64ish")]
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub suffix: Option<String>,
    #[serde(default, deserialize_with = "opt_u32ish")]
    pub bit_rate: Option<u32>,
    pub genre: Option<String>,
    pub created: Option<String>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub starred: Option<String>,
    #[serde(default, deserialize_with = "opt_boolish")]
    pub is_dir: Option<bool>,
    #[serde(default, deserialize_with = "opt_stringish")]
    pub media_type: Option<String>,
}

impl From<RawSong> for SongResponse {
    fn from(value: RawSong) -> Self {
        let cover_art_id = value.cover_art;
        Self {
            id: value.id,
            parent_id: value.parent,
            path: value.path,
            title: value.title,
            album: value.album,
            album_id: value.album_id,
            artist: value.artist,
            artist_id: value.artist_id,
            display_cover_art_id: cover_art_id.clone(),
            cover_art_id,
            track: value.track,
            disc_number: value.disc_number,
            year: value.year,
            duration: value.duration,
            size: value.size,
            content_type: value.content_type,
            suffix: value.suffix,
            bit_rate: value.bit_rate,
            genre: value.genre,
            created: value.created,
            starred: value.starred,
            is_directory: value.is_dir.unwrap_or(false),
            media_type: normalize_media_type(value.media_type),
        }
    }
}
