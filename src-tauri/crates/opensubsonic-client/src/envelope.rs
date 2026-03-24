use serde::Deserialize;

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope<T> {
    pub meta: ResponseMeta,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseMeta {
    pub status: ResponseStatus,
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatus {
    Ok,
    Failed,
    Unknown(String),
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawEnvelope<T> {
    #[serde(rename = "subsonic-response")]
    pub response: RawResponse<T>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawResponse<T> {
    pub status: String,
    pub version: String,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
    #[serde(rename = "serverVersion")]
    pub server_version: Option<String>,
    #[serde(rename = "openSubsonic")]
    pub open_subsonic: Option<bool>,
    pub error: Option<RawError>,
    #[serde(flatten)]
    pub payload: T,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawError {
    pub code: u32,
    pub message: Option<String>,
    #[serde(rename = "helpUrl")]
    pub help_url: Option<String>,
}

impl<T> RawEnvelope<T> {
    pub(crate) fn into_envelope(self) -> Result<Envelope<T>, ApiError> {
        let meta = ResponseMeta {
            status: ResponseStatus::from_raw(&self.response.status),
            api_version: self.response.version.clone(),
            server_type: self.response.server_type.clone(),
            server_version: self.response.server_version.clone(),
            open_subsonic: self.response.open_subsonic,
        };

        if matches!(meta.status, ResponseStatus::Ok) {
            return Ok(Envelope {
                meta,
                payload: self.response.payload,
            });
        }

        let Some(error) = self.response.error else {
            return Err(ApiError::Protocol(
                "The server reported a failure without an error payload.".to_string(),
            ));
        };

        Err(ApiError::Api {
            code: error.code,
            message: error.message,
            help_url: error.help_url,
            meta,
        })
    }
}

impl ResponseStatus {
    fn from_raw(value: &str) -> Self {
        if value.eq_ignore_ascii_case("ok") {
            Self::Ok
        } else if value.eq_ignore_ascii_case("failed") {
            Self::Failed
        } else {
            Self::Unknown(value.to_string())
        }
    }
}
