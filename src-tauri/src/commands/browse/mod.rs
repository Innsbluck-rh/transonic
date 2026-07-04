pub mod album;
pub mod artist;
pub mod folder_structure;
pub mod folder_structure_album_songs;
pub mod song;

use crate::commands::common::normalize_media_type;
use crate::models::SongResponse;

impl From<opensubsonic_client::Child> for SongResponse {
    fn from(value: opensubsonic_client::Child) -> Self {
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
