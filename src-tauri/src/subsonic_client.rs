use std::time::Duration;

use async_trait::async_trait;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

use crate::models::{
    ActiveSession, AuthInput, AuthKind, CapabilityMatrix, ConnectServerProfileResult,
    LastConnectionState, OpenSubsonicExtension, RestoreStatus, SavedProfileSummary,
};

const API_VERSION: &str = "1.13.0";
const CLIENT_NAME: &str = "transonic";
const REQUEST_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone)]
pub struct ConnectedServer {
    pub normalized_server_url: String,
    pub username: String,
    pub auth_kind: AuthKind,
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub capability_matrix: CapabilityMatrix,
}

impl ConnectedServer {
    pub fn into_active_session(self, profile_id: String) -> ActiveSession {
        ActiveSession {
            profile_id,
            normalized_server_url: self.normalized_server_url,
            username: self.username,
            auth_kind: self.auth_kind,
            api_version: self.api_version,
            server_type: self.server_type,
            server_version: self.server_version,
            capability_matrix: self.capability_matrix,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectFailure {
    Auth {
        message: String,
        code: Option<u32>,
        help_url: Option<String>,
    },
    Network {
        message: String,
    },
    Server {
        message: String,
        code: Option<u32>,
        help_url: Option<String>,
    },
    UnsupportedAuth {
        message: String,
    },
}

impl ConnectFailure {
    pub fn last_connection_state(&self) -> LastConnectionState {
        match self {
            Self::Auth { .. } | Self::UnsupportedAuth { .. } => LastConnectionState::ReauthRequired,
            Self::Network { .. } | Self::Server { .. } => LastConnectionState::Offline,
        }
    }

    pub fn restore_status(&self) -> RestoreStatus {
        match self {
            Self::Auth { .. } | Self::UnsupportedAuth { .. } => RestoreStatus::ReauthRequired,
            Self::Network { .. } | Self::Server { .. } => RestoreStatus::Offline,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Auth { message, .. }
            | Self::Network { message }
            | Self::Server { message, .. }
            | Self::UnsupportedAuth { message } => message,
        }
    }

    pub fn into_public_result(
        self,
        profiles: Vec<SavedProfileSummary>,
        active_session: Option<ActiveSession>,
    ) -> ConnectServerProfileResult {
        match self {
            Self::Auth {
                message,
                code,
                help_url,
            } => ConnectServerProfileResult::AuthError {
                message,
                code,
                help_url,
                active_session,
                profiles,
            },
            Self::Network { message } => ConnectServerProfileResult::NetworkError {
                message,
                active_session,
                profiles,
            },
            Self::Server {
                message,
                code,
                help_url,
            } => ConnectServerProfileResult::ServerError {
                message,
                code,
                help_url,
                active_session,
                profiles,
            },
            Self::UnsupportedAuth { message } => ConnectServerProfileResult::UnsupportedAuth {
                message,
                active_session,
                profiles,
            },
        }
    }
}

#[async_trait]
pub trait SubsonicApi: Send + Sync {
    async fn connect(
        &self,
        server_url: &str,
        auth: &AuthInput,
    ) -> Result<ConnectedServer, ConnectFailure>;
}

#[derive(Debug, Clone)]
pub struct ReqwestSubsonicClient {
    client: Client,
}

impl ReqwestSubsonicClient {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .map_err(|error| format!("Failed to prepare the HTTP client: {error}"))?;

        Ok(Self { client })
    }

    pub async fn fetch_capabilities(&self, normalized_server_url: &str) -> CapabilityMatrix {
        let url = format!("{normalized_server_url}/getOpenSubsonicExtensions.view");
        let response = match self.client.get(&url).query(&standard_query()).send().await {
            Ok(response) => response,
            Err(_) => return CapabilityMatrix::empty(),
        };

        if matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::NOT_IMPLEMENTED
        ) {
            return CapabilityMatrix::empty();
        }

        let body = match response.text().await {
            Ok(body) => body,
            Err(_) => return CapabilityMatrix::empty(),
        };

        let envelope = match serde_json::from_str::<SubsonicEnvelope<ExtensionsResponse>>(&body) {
            Ok(envelope) => envelope,
            Err(_) => return CapabilityMatrix::empty(),
        };

        if !envelope.response.status.eq_ignore_ascii_case("ok") {
            return CapabilityMatrix::empty();
        }

        CapabilityMatrix::from_extensions(
            envelope.response.open_subsonic.unwrap_or(false),
            envelope
                .response
                .open_subsonic_extensions
                .unwrap_or_default(),
        )
    }

    async fn ping(
        &self,
        normalized_server_url: &str,
        auth: &AuthInput,
    ) -> Result<SubsonicResponse, ConnectFailure> {
        let ping_url = format!("{normalized_server_url}/ping.view");
        let response = self
            .client
            .get(&ping_url)
            .query(&request_query(auth))
            .send()
            .await
            .map_err(|error| ConnectFailure::Network {
                message: describe_network_error(error),
            })?;

        let http_status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| ConnectFailure::Network {
                message: format!("Failed to read the server response: {error}"),
            })?;

        match serde_json::from_str::<SubsonicEnvelope<SubsonicResponse>>(&body) {
            Ok(envelope) => classify_subsonic_response(envelope.response),
            Err(_)
                if matches!(
                    http_status,
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                ) =>
            {
                Err(ConnectFailure::Auth {
                    message:
                        "The server rejected the credentials before returning a Subsonic response."
                            .to_string(),
                    code: None,
                    help_url: None,
                })
            }
            Err(_) => Err(ConnectFailure::Server {
                message: format!(
                    "The server returned HTTP {} without a valid Subsonic JSON response.",
                    http_status.as_u16()
                ),
                code: None,
                help_url: None,
            }),
        }
    }

    async fn fetch_token_username(
        &self,
        normalized_server_url: &str,
        api_key: &str,
    ) -> Option<String> {
        let url = format!("{normalized_server_url}/tokenInfo.view");
        let mut query = vec![("apiKey".to_string(), api_key.to_string())];
        query.extend(standard_query());

        let response = self.client.get(&url).query(&query).send().await.ok()?;
        let body = response.text().await.ok()?;
        let envelope = serde_json::from_str::<SubsonicEnvelope<TokenInfoResponse>>(&body).ok()?;

        if !envelope.response.status.eq_ignore_ascii_case("ok") {
            return None;
        }

        envelope
            .response
            .token_info
            .map(|token_info| token_info.username)
    }
}

#[async_trait]
impl SubsonicApi for ReqwestSubsonicClient {
    async fn connect(
        &self,
        server_url: &str,
        auth: &AuthInput,
    ) -> Result<ConnectedServer, ConnectFailure> {
        let normalized_server_url = normalize_server_url(server_url)
            .map_err(|message| ConnectFailure::Network { message })?;

        let username = auth.username().trim().to_string();
        if username.is_empty() || auth.secret().is_empty() {
            return Err(ConnectFailure::Auth {
                message: "Username and credentials are required.".to_string(),
                code: None,
                help_url: None,
            });
        }

        let capability_matrix = self.fetch_capabilities(&normalized_server_url).await;
        if matches!(auth, AuthInput::ApiKey { .. }) && !capability_matrix.api_key_auth {
            return Err(ConnectFailure::UnsupportedAuth {
                message: "This server does not advertise OpenSubsonic API key authentication."
                    .to_string(),
            });
        }

        let response = self.ping(&normalized_server_url, auth).await?;
        let capability_matrix =
            capability_matrix.merge_open_subsonic(response.open_subsonic.unwrap_or(false));

        let resolved_username = match auth {
            AuthInput::Password { .. } => username,
            AuthInput::ApiKey { api_key, .. } => self
                .fetch_token_username(&normalized_server_url, api_key)
                .await
                .unwrap_or(username),
        };

        Ok(ConnectedServer {
            normalized_server_url,
            username: resolved_username,
            auth_kind: auth.auth_kind(),
            api_version: response.version,
            server_type: response.server_type,
            server_version: response.server_version,
            capability_matrix,
        })
    }
}

#[derive(Debug, Deserialize)]
struct SubsonicEnvelope<T> {
    #[serde(rename = "subsonic-response")]
    response: T,
}

#[derive(Debug, Deserialize)]
struct SubsonicResponse {
    status: String,
    version: String,
    #[serde(rename = "type")]
    server_type: Option<String>,
    #[serde(rename = "serverVersion")]
    server_version: Option<String>,
    #[serde(rename = "openSubsonic")]
    open_subsonic: Option<bool>,
    error: Option<SubsonicError>,
}

#[derive(Debug, Deserialize)]
struct ExtensionsResponse {
    status: String,
    #[serde(rename = "openSubsonic")]
    open_subsonic: Option<bool>,
    #[serde(rename = "openSubsonicExtensions")]
    open_subsonic_extensions: Option<Vec<OpenSubsonicExtension>>,
}

#[derive(Debug, Deserialize)]
struct TokenInfoResponse {
    status: String,
    #[serde(rename = "tokenInfo")]
    token_info: Option<TokenInfoPayload>,
}

#[derive(Debug, Deserialize)]
struct TokenInfoPayload {
    username: String,
}

#[derive(Debug, Deserialize)]
struct SubsonicError {
    code: u32,
    message: Option<String>,
    #[serde(rename = "helpUrl")]
    help_url: Option<String>,
}

fn classify_subsonic_response(
    response: SubsonicResponse,
) -> Result<SubsonicResponse, ConnectFailure> {
    if response.status.eq_ignore_ascii_case("ok") {
        return Ok(response);
    }

    let Some(error) = response.error else {
        return Err(ConnectFailure::Server {
            message: "The server reported a failure without an error payload.".to_string(),
            code: None,
            help_url: None,
        });
    };

    let message = error
        .message
        .unwrap_or_else(|| "The server rejected the request.".to_string());

    match classify_error_code(error.code) {
        ErrorCategory::Auth => Err(ConnectFailure::Auth {
            message,
            code: Some(error.code),
            help_url: error.help_url,
        }),
        ErrorCategory::Server => Err(ConnectFailure::Server {
            message,
            code: Some(error.code),
            help_url: error.help_url,
        }),
    }
}

pub fn normalize_server_url(server_url: &str) -> Result<String, String> {
    let trimmed = server_url.trim();
    if trimmed.is_empty() {
        return Err("Server URL is required.".to_string());
    }

    let mut url = Url::parse(trimmed)
        .map_err(|_| "Enter a valid server URL starting with http:// or https://.".to_string())?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err("Server URL must start with http:// or https://.".to_string());
    }

    if url.host_str().is_none() {
        return Err("Server URL must include a host name.".to_string());
    }

    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);

    let path = url.path().trim_end_matches('/');
    let normalized_path = if path.is_empty() {
        "/rest".to_string()
    } else if path.ends_with("/rest") {
        path.to_string()
    } else {
        format!("{path}/rest")
    };

    url.set_path(&normalized_path);
    Ok(url.to_string())
}

fn request_query(auth: &AuthInput) -> Vec<(String, String)> {
    let mut query = match auth {
        AuthInput::Password { username, password } => {
            build_password_auth_query(username.trim(), password, &generate_salt())
        }
        AuthInput::ApiKey { api_key, .. } => build_api_key_auth_query(api_key),
    };
    query.extend(standard_query());
    query
}

pub fn build_password_auth_query(
    username: &str,
    password: &str,
    salt: &str,
) -> Vec<(String, String)> {
    vec![
        ("u".to_string(), username.to_string()),
        ("t".to_string(), build_token(password, salt)),
        ("s".to_string(), salt.to_string()),
    ]
}

pub fn build_api_key_auth_query(api_key: &str) -> Vec<(String, String)> {
    vec![("apiKey".to_string(), api_key.to_string())]
}

fn standard_query() -> Vec<(String, String)> {
    vec![
        ("v".to_string(), API_VERSION.to_string()),
        ("c".to_string(), CLIENT_NAME.to_string()),
        ("f".to_string(), "json".to_string()),
    ]
}

fn generate_salt() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

pub fn build_token(password: &str, salt: &str) -> String {
    format!("{:x}", md5::compute(format!("{password}{salt}")))
}

enum ErrorCategory {
    Auth,
    Server,
}

fn classify_error_code(code: u32) -> ErrorCategory {
    match code {
        40..=44 => ErrorCategory::Auth,
        _ => ErrorCategory::Server,
    }
}

fn describe_network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        return "The connection attempt timed out.".to_string();
    }

    if error.is_connect() {
        return format!("Could not reach the server: {error}");
    }

    if error.is_request() {
        return format!("The request could not be built or sent: {error}");
    }

    format!("The connection attempt failed: {error}")
}

#[cfg(test)]
mod tests {
    use mockito::{Matcher, Server};

    use super::{
        build_api_key_auth_query, build_password_auth_query, build_token, normalize_server_url,
        ConnectFailure, ReqwestSubsonicClient, SubsonicApi,
    };
    use crate::models::{AuthInput, CapabilityMatrix};

    #[test]
    fn normalize_server_url_adds_rest_to_root_urls() {
        let normalized = normalize_server_url("https://demo.example").unwrap();
        assert_eq!(normalized, "https://demo.example/rest");
    }

    #[test]
    fn normalize_server_url_keeps_existing_rest_path() {
        let normalized = normalize_server_url("https://demo.example/subsonic/rest/").unwrap();
        assert_eq!(normalized, "https://demo.example/subsonic/rest");
    }

    #[test]
    fn normalize_server_url_drops_query_and_fragment() {
        let normalized =
            normalize_server_url("https://demo.example/subsonic?token=123#ignore").unwrap();
        assert_eq!(normalized, "https://demo.example/subsonic/rest");
    }

    #[test]
    fn normalize_server_url_removes_embedded_credentials() {
        let normalized = normalize_server_url("https://demo:secret@example.com/player").unwrap();
        assert_eq!(normalized, "https://example.com/player/rest");
    }

    #[test]
    fn build_token_matches_the_subsonic_documentation_example() {
        let token = build_token("sesame", "c19b2d");
        assert_eq!(token, "26719a1196d2a940705a59634eb18eab");
    }

    #[test]
    fn build_password_auth_query_uses_token_authentication() {
        let query = build_password_auth_query("demo", "sesame", "c19b2d");

        assert!(query.contains(&("u".to_string(), "demo".to_string())));
        assert!(query.contains(&(
            "t".to_string(),
            "26719a1196d2a940705a59634eb18eab".to_string()
        )));
        assert!(query.contains(&("s".to_string(), "c19b2d".to_string())));
    }

    #[test]
    fn build_api_key_auth_query_only_uses_the_api_key() {
        let query = build_api_key_auth_query("secret-key");

        assert_eq!(
            query,
            vec![("apiKey".to_string(), "secret-key".to_string())]
        );
    }

    #[tokio::test]
    async fn fetch_capabilities_accepts_open_subsonic_extensions() {
        let mut server = Server::new_async().await;
        let body = r#"{
          "subsonic-response": {
            "status": "ok",
            "openSubsonic": true,
            "openSubsonicExtensions": [
              { "name": "apiKeyAuthentication", "versions": [1] },
              { "name": "playbackReport", "versions": [1] }
            ]
          }
        }"#;

        server
            .mock("GET", "/rest/getOpenSubsonicExtensions.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("v".into(), "1.13.0".into()),
                Matcher::UrlEncoded("c".into(), "transonic".into()),
                Matcher::UrlEncoded("f".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let client = ReqwestSubsonicClient::new().unwrap();
        let capabilities = client
            .fetch_capabilities(&format!("{}/rest", server.url()))
            .await;

        assert!(capabilities.open_subsonic);
        assert!(capabilities.api_key_auth);
        assert!(capabilities.playback_report);
    }

    #[tokio::test]
    async fn fetch_capabilities_treats_404_as_non_open_subsonic() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/rest/getOpenSubsonicExtensions.view")
            .with_status(404)
            .create_async()
            .await;

        let client = ReqwestSubsonicClient::new().unwrap();
        let capabilities = client
            .fetch_capabilities(&format!("{}/rest", server.url()))
            .await;

        assert_eq!(capabilities.raw_extensions.len(), 0);
        assert!(!capabilities.open_subsonic);
    }

    #[tokio::test]
    async fn fetch_capabilities_treats_non_json_as_non_open_subsonic() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/rest/getOpenSubsonicExtensions.view")
            .with_status(200)
            .with_body("this is not json")
            .create_async()
            .await;

        let client = ReqwestSubsonicClient::new().unwrap();
        let capabilities = client
            .fetch_capabilities(&format!("{}/rest", server.url()))
            .await;

        assert_eq!(capabilities.raw_extensions.len(), 0);
        assert!(!capabilities.open_subsonic);
    }

    #[tokio::test]
    async fn connect_rejects_api_key_auth_when_server_does_not_advertise_it() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/rest/getOpenSubsonicExtensions.view")
            .with_status(404)
            .create_async()
            .await;

        let client = ReqwestSubsonicClient::new().unwrap();
        let result = client
            .connect(
                &server.url(),
                &AuthInput::ApiKey {
                    username: "demo".to_string(),
                    api_key: "secret".to_string(),
                },
            )
            .await;

        match result {
            Err(ConnectFailure::UnsupportedAuth { .. }) => {}
            other => panic!("expected unsupported auth, got {other:?}"),
        }
    }

    #[test]
    fn capability_matrix_empty_disables_all_known_flags() {
        let matrix = CapabilityMatrix::empty();

        assert!(!matrix.api_key_auth);
        assert!(!matrix.index_based_queue);
        assert!(!matrix.playback_report);
        assert!(!matrix.transcoding);
        assert!(!matrix.transcode_offset);
        assert!(!matrix.song_lyrics);
    }
}
