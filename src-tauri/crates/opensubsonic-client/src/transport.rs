use std::time::Duration;

use reqwest::{header::HeaderMap, Client, Url};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    auth::Auth,
    config::ApiVersion,
    envelope::{Envelope, RawEnvelope},
    error::ApiError,
};

const JSON_TIMEOUT_SECONDS: u64 = 10;
const BINARY_TIMEOUT_SECONDS: u64 = 60;
/// Timeout for the TCP/TLS connection phase only. Kept short so an unreachable
/// server (e.g. a home server behind a VPN that is currently disconnected)
/// fails fast instead of hanging on the full request timeout. A healthy server
/// completes the handshake well under this budget, so normal requests are
/// unaffected.
const CONNECT_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    json_client: Client,
    binary_client: Client,
}

#[derive(Debug, Clone)]
pub struct PreparedBinaryRequest {
    pub url: Url,
    pub headers: HeaderMap,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, ApiError> {
        let json_client = Client::builder()
            .timeout(Duration::from_secs(JSON_TIMEOUT_SECONDS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
            .build()
            .map_err(|error| ApiError::ClientBuild(error.to_string()))?;
        let binary_client = Client::builder()
            .timeout(Duration::from_secs(BINARY_TIMEOUT_SECONDS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
            .build()
            .map_err(|error| ApiError::ClientBuild(error.to_string()))?;

        Ok(Self {
            json_client,
            binary_client,
        })
    }

    pub async fn send_json_get<Q, P>(
        &self,
        base_url: &Url,
        endpoint: &str,
        auth: Option<&Auth>,
        api_version: &ApiVersion,
        client_name: &str,
        query: &Q,
    ) -> Result<Envelope<P>, ApiError>
    where
        Q: Serialize,
        P: DeserializeOwned,
    {
        let url = endpoint_url(base_url, endpoint);
        let pairs = build_query_pairs(auth, api_version, client_name, query)?;
        let response = self
            .json_client
            .get(url)
            .query(&pairs)
            .send()
            .await
            .map_err(ApiError::Transport)?;

        parse_json_response(response).await
    }

    pub async fn send_json_post<Q, B, P>(
        &self,
        base_url: &Url,
        endpoint: &str,
        auth: Option<&Auth>,
        api_version: &ApiVersion,
        client_name: &str,
        query: &Q,
        body: &B,
    ) -> Result<Envelope<P>, ApiError>
    where
        Q: Serialize,
        B: Serialize,
        P: DeserializeOwned,
    {
        let url = endpoint_url(base_url, endpoint);
        let pairs = build_query_pairs(auth, api_version, client_name, query)?;
        let response = self
            .json_client
            .post(url)
            .query(&pairs)
            .json(body)
            .send()
            .await
            .map_err(ApiError::Transport)?;

        parse_json_response(response).await
    }

    pub fn prepare_binary_get<Q>(
        &self,
        base_url: &Url,
        endpoint: &str,
        auth: &Auth,
        api_version: &ApiVersion,
        client_name: &str,
        query: &Q,
    ) -> Result<PreparedBinaryRequest, ApiError>
    where
        Q: Serialize,
    {
        let url = endpoint_url(base_url, endpoint);
        let pairs = build_query_pairs(Some(auth), api_version, client_name, query)?;
        let request = self
            .binary_client
            .get(url)
            .query(&pairs)
            .build()
            .map_err(ApiError::Transport)?;

        Ok(PreparedBinaryRequest {
            url: request.url().clone(),
            headers: request.headers().clone(),
        })
    }
}

async fn parse_json_response<P>(response: reqwest::Response) -> Result<Envelope<P>, ApiError>
where
    P: DeserializeOwned,
{
    let http_status = response.status();
    let body = response.text().await.map_err(ApiError::Transport)?;

    match serde_json::from_str::<RawEnvelope<Value>>(&body) {
        Ok(envelope) => {
            let envelope = envelope.into_envelope()?;
            let payload = serde_json::from_value::<P>(envelope.payload).map_err(|error| {
                ApiError::Decode {
                    message: error.to_string(),
                    body_preview: Some(preview_body(&body)),
                }
            })?;

            Ok(Envelope {
                meta: envelope.meta,
                payload,
            })
        }
        Err(_error) if !http_status.is_success() => Err(ApiError::HttpStatus {
            status: http_status,
            body_preview: Some(preview_body(&body)),
        }),
        Err(error) => Err(ApiError::Decode {
            message: error.to_string(),
            body_preview: Some(preview_body(&body)),
        }),
    }
}

pub(crate) fn endpoint_url(base_url: &Url, endpoint: &str) -> Url {
    let mut url = base_url.clone();
    let mut path = url.path().trim_end_matches('/').to_string();
    path.push('/');
    path.push_str(endpoint);
    path.push_str(".view");
    url.set_path(&path);
    url
}

fn build_query_pairs<Q>(
    auth: Option<&Auth>,
    api_version: &ApiVersion,
    client_name: &str,
    query: &Q,
) -> Result<Vec<(String, String)>, ApiError>
where
    Q: Serialize,
{
    let mut pairs = Vec::new();
    if let Some(auth) = auth {
        pairs.extend(auth.to_query_pairs());
    }
    pairs.push(("v".to_string(), api_version.as_str().to_string()));
    pairs.push(("c".to_string(), client_name.to_string()));
    pairs.push(("f".to_string(), "json".to_string()));
    pairs.extend(value_to_query_pairs(serde_json::to_value(query).map_err(
        |error| ApiError::Protocol(format!("Failed to serialize query parameters: {error}")),
    )?));
    Ok(pairs)
}

fn value_to_query_pairs(value: Value) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Value::Object(map) = value {
        for (key, value) in map {
            append_query_value(&mut pairs, &key, value);
        }
    }
    pairs
}

fn append_query_value(pairs: &mut Vec<(String, String)>, key: &str, value: Value) {
    match value {
        Value::Null => {}
        Value::Bool(value) => pairs.push((key.to_string(), value.to_string())),
        Value::Number(value) => pairs.push((key.to_string(), value.to_string())),
        Value::String(value) => pairs.push((key.to_string(), value)),
        Value::Array(values) => {
            for value in values {
                append_query_value(pairs, key, value);
            }
        }
        Value::Object(_) => {
            pairs.push((key.to_string(), value.to_string()));
        }
    }
}

fn preview_body(body: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 256;
    body.chars().take(MAX_PREVIEW_CHARS).collect()
}
