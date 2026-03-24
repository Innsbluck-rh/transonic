use serde::{de::DeserializeOwned, Serialize};

use crate::{
    config::ClientConfig,
    envelope::Envelope,
    error::ApiError,
    transport::{PreparedBinaryRequest, ReqwestTransport},
};

#[derive(Debug, Clone)]
pub struct OpenSubsonicClient {
    config: ClientConfig,
    transport: ReqwestTransport,
}

impl OpenSubsonicClient {
    pub fn new(config: ClientConfig) -> Result<Self, ApiError> {
        Ok(Self {
            config,
            transport: ReqwestTransport::new()?,
        })
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub(crate) fn transport(&self) -> &ReqwestTransport {
        &self.transport
    }

    pub(crate) async fn json_get<Q, P>(
        &self,
        endpoint: &str,
        query: &Q,
    ) -> Result<Envelope<P>, ApiError>
    where
        Q: Serialize + Send + Sync,
        P: DeserializeOwned,
    {
        self.transport()
            .send_json_get(
                &self.config().base_url,
                endpoint,
                Some(&self.config().auth),
                &self.config().api_version,
                &self.config().client_name,
                query,
            )
            .await
    }

    pub(crate) fn binary_get<Q>(
        &self,
        endpoint: &str,
        query: &Q,
    ) -> Result<PreparedBinaryRequest, ApiError>
    where
        Q: Serialize,
    {
        self.transport().prepare_binary_get(
            &self.config().base_url,
            endpoint,
            &self.config().auth,
            &self.config().api_version,
            &self.config().client_name,
            query,
        )
    }
}
