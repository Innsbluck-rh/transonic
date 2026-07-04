use std::collections::BTreeMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Child {
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

pub(crate) fn value_as<T>(value: Value) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
}

pub(crate) fn vec_or_single<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| value_as(item).map_err(serde::de::Error::custom))
            .collect(),
        Value::Object(_) => Ok(vec![value_as(value).map_err(serde::de::Error::custom)?]),
        Value::Null => Ok(Vec::new()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected collection payload: {other}"
        ))),
    }
}

pub(crate) fn stringish<'de, D>(deserializer: D) -> Result<String, D::Error>
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

pub(crate) fn opt_stringish<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
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

pub(crate) fn opt_boolish<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
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

pub(crate) fn opt_u64ish<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(None),
        Value::Number(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("unexpected number payload: {value}"))),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            trimmed.parse::<u64>().map(Some).map_err(|error| {
                serde::de::Error::custom(format!("unexpected number payload: {error}"))
            })
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected number payload: {other}"
        ))),
    }
}

pub(crate) fn opt_u32ish<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
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
            .ok_or_else(|| serde::de::Error::custom(format!("unexpected number payload: {value}"))),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            trimmed.parse::<u32>().map(Some).map_err(|error| {
                serde::de::Error::custom(format!("unexpected number payload: {error}"))
            })
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected number payload: {other}"
        ))),
    }
}
