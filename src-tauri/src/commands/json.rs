use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

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
