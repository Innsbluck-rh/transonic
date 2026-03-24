use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::OpenSubsonicClient,
    envelope::Envelope,
    error::ApiError,
    transport::PreparedBinaryRequest,
    types::common::{ClientInfo, MediaType},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetTranscodeDecisionRequest {
    pub media_id: String,
    pub media_type: MediaType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetTranscodeDecisionResponse {
    #[serde(rename = "transcodeDecision")]
    pub transcode_decision: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetTranscodeStreamRequest {
    pub media_id: String,
    pub media_type: MediaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    pub transcode_params: String,
}

#[async_trait]
pub trait TranscodingApi {
    async fn get_transcode_decision(
        &self,
        req: GetTranscodeDecisionRequest,
        body: ClientInfo,
    ) -> Result<Envelope<GetTranscodeDecisionResponse>, ApiError>;
    fn get_transcode_stream(
        &self,
        req: GetTranscodeStreamRequest,
    ) -> Result<PreparedBinaryRequest, ApiError>;
}

#[async_trait]
impl TranscodingApi for OpenSubsonicClient {
    async fn get_transcode_decision(
        &self,
        req: GetTranscodeDecisionRequest,
        body: ClientInfo,
    ) -> Result<Envelope<GetTranscodeDecisionResponse>, ApiError> {
        self.transport()
            .send_json_post(
                &self.config().base_url,
                "getTranscodeDecision",
                Some(&self.config().auth),
                &self.config().api_version,
                &self.config().client_name,
                &req,
                &body,
            )
            .await
    }

    fn get_transcode_stream(
        &self,
        req: GetTranscodeStreamRequest,
    ) -> Result<PreparedBinaryRequest, ApiError> {
        self.binary_get("getTranscodeStream", &req)
    }
}
