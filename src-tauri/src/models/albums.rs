use opensubsonic_client::{api::lists::GetAlbumList2Request, AlbumListType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AlbumListContext {
    Random,
    Newest,
    Frequent,
    Recent,
    Starred,
    AlphabeticalByName,
    AlphabeticalByArtist,
    ByYear,
    ByGenre,
}

impl AlbumListContext {
    pub fn to_open_subsonic_type(&self) -> AlbumListType {
        match self {
            Self::Random => AlbumListType::Random,
            Self::Newest => AlbumListType::Newest,
            Self::Frequent => AlbumListType::Frequent,
            Self::Recent => AlbumListType::Recent,
            Self::Starred => AlbumListType::Starred,
            Self::AlphabeticalByName => AlbumListType::AlphabeticalByName,
            Self::AlphabeticalByArtist => AlbumListType::AlphabeticalByArtist,
            Self::ByYear => AlbumListType::ByYear,
            Self::ByGenre => AlbumListType::ByGenre,
        }
    }
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListRequest {
    pub context: AlbumListContext,
    pub size: Option<u32>,
    pub offset: Option<u32>,
    pub from_year: Option<u32>,
    pub to_year: Option<u32>,
    pub genre: Option<String>,
    pub music_folder_id: Option<String>,
}

impl AlbumListRequest {
    pub fn to_open_subsonic_request(&self) -> Result<GetAlbumList2Request, String> {
        match self.context {
            AlbumListContext::ByYear if self.from_year.is_none() || self.to_year.is_none() => {
                Err("The by_year album context requires both fromYear and toYear.".to_string())
            }
            AlbumListContext::ByGenre if self.genre.as_deref().unwrap_or("").trim().is_empty() => {
                Err("The by_genre album context requires a non-empty genre.".to_string())
            }
            _ => Ok(GetAlbumList2Request {
                list_type: self.context.to_open_subsonic_type(),
                size: self.size,
                offset: self.offset,
                from_year: self.from_year,
                to_year: self.to_year,
                genre: self.genre.clone(),
                music_folder_id: self.music_folder_id.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListItem {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art_id: Option<String>,
    pub song_count: Option<u32>,
    pub duration: Option<u32>,
    pub play_count: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub created: Option<String>,
    pub starred: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListResponse {
    pub context: AlbumListContext,
    pub albums: Vec<AlbumListItem>,
}
