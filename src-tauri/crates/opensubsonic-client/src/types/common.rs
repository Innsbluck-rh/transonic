use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EmptyPayload {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Song,
    Podcast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Starting,
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlbumListType {
    #[serde(rename = "random")]
    Random,
    #[serde(rename = "newest")]
    Newest,
    #[serde(rename = "highest")]
    Highest,
    #[serde(rename = "frequent")]
    Frequent,
    #[serde(rename = "recent")]
    Recent,
    #[serde(rename = "alphabeticalByName")]
    AlphabeticalByName,
    #[serde(rename = "alphabeticalByArtist")]
    AlphabeticalByArtist,
    #[serde(rename = "starred")]
    Starred,
    #[serde(rename = "byYear")]
    ByYear,
    #[serde(rename = "byGenre")]
    ByGenre,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    #[serde(flatten)]
    pub values: BTreeMap<String, Value>,
}
