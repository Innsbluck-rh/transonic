use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::{
    config::ApiVersion,
    envelope::Envelope,
    error::ApiError,
    extensions::{OpenSubsonicExtension, ServerFeatures},
    transport::ReqwestTransport,
    types::common::EmptyPayload,
};

#[derive(Debug, Clone)]
pub struct ServerProbe {
    transport: ReqwestTransport,
    client_name: String,
    api_version: ApiVersion,
}

impl ServerProbe {
    pub fn new(client_name: impl Into<String>) -> Result<Self, ApiError> {
        Ok(Self {
            transport: ReqwestTransport::new()?,
            client_name: client_name.into(),
            api_version: ApiVersion::default(),
        })
    }

    pub async fn get_open_subsonic_extensions(
        &self,
        base_url: &Url,
    ) -> Result<Envelope<GetOpenSubsonicExtensionsResponse>, ApiError> {
        self.transport
            .send_json_get(
                base_url,
                "getOpenSubsonicExtensions",
                None,
                &self.api_version,
                &self.client_name,
                &EmptyPayload::default(),
            )
            .await
    }

    pub async fn fetch_server_features(&self, base_url: &Url) -> Result<ServerFeatures, ApiError> {
        match self.get_open_subsonic_extensions(base_url).await {
            Ok(response) => Ok(ServerFeatures::from_extensions(
                response.meta.open_subsonic.unwrap_or(false),
                response.payload.open_subsonic_extensions,
            )),
            Err(ApiError::HttpStatus { status, .. })
                if matches!(status, StatusCode::NOT_FOUND | StatusCode::NOT_IMPLEMENTED) =>
            {
                Ok(ServerFeatures::empty())
            }
            Err(ApiError::Decode { .. }) => Ok(ServerFeatures::empty()),
            Err(error) => Err(error),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GetOpenSubsonicExtensionsResponse {
    #[serde(default, rename = "openSubsonicExtensions")]
    pub open_subsonic_extensions: Vec<OpenSubsonicExtension>,
}
