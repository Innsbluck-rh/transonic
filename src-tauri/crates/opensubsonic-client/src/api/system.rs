use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::OpenSubsonicClient, envelope::Envelope, error::ApiError,
    probe::GetOpenSubsonicExtensionsResponse, types::common::EmptyPayload,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PingResponse {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetLicenseResponse {
    #[serde(rename = "license")]
    pub license: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfoResponse {
    #[serde(rename = "tokenInfo")]
    pub token_info: TokenInfo,
}

#[async_trait]
pub trait SystemApi {
    async fn ping(&self) -> Result<Envelope<PingResponse>, ApiError>;
    async fn get_license(&self) -> Result<Envelope<GetLicenseResponse>, ApiError>;
    async fn get_open_subsonic_extensions(
        &self,
    ) -> Result<Envelope<GetOpenSubsonicExtensionsResponse>, ApiError>;
    async fn token_info(&self) -> Result<Envelope<TokenInfoResponse>, ApiError>;
}

#[async_trait]
impl SystemApi for OpenSubsonicClient {
    async fn ping(&self) -> Result<Envelope<PingResponse>, ApiError> {
        self.transport()
            .send_json_get(
                &self.config().base_url,
                "ping",
                Some(&self.config().auth),
                &self.config().api_version,
                &self.config().client_name,
                &EmptyPayload::default(),
            )
            .await
    }

    async fn get_license(&self) -> Result<Envelope<GetLicenseResponse>, ApiError> {
        self.transport()
            .send_json_get(
                &self.config().base_url,
                "getLicense",
                Some(&self.config().auth),
                &self.config().api_version,
                &self.config().client_name,
                &EmptyPayload::default(),
            )
            .await
    }

    async fn get_open_subsonic_extensions(
        &self,
    ) -> Result<Envelope<GetOpenSubsonicExtensionsResponse>, ApiError> {
        self.transport()
            .send_json_get(
                &self.config().base_url,
                "getOpenSubsonicExtensions",
                None,
                &self.config().api_version,
                &self.config().client_name,
                &EmptyPayload::default(),
            )
            .await
    }

    async fn token_info(&self) -> Result<Envelope<TokenInfoResponse>, ApiError> {
        self.transport()
            .send_json_get(
                &self.config().base_url,
                "tokenInfo",
                Some(&self.config().auth),
                &self.config().api_version,
                &self.config().client_name,
                &EmptyPayload::default(),
            )
            .await
    }
}
