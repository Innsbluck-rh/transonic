use async_trait::async_trait;
use opensubsonic_client::{
    api::system::SystemApi, normalize_base_url, ApiError, Auth, ClientConfig,
    OpenSubsonicClient, ServerFeatures, ServerProbe,
};
use reqwest::StatusCode;

use crate::models::{
    ActiveSession, AuthInput, AuthKind, CapabilityMatrix, ConnectServerProfileResult,
    LastConnectionState, OpenSubsonicExtension, RestoreStatus, SavedProfileSummary,
};

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
pub trait ConnectionApi: Send + Sync {
    async fn connect(
        &self,
        server_url: &str,
        auth: &AuthInput,
    ) -> Result<ConnectedServer, ConnectFailure>;

    #[allow(dead_code)]
    fn build_client_from_active_session(
        &self,
        session: &ActiveSession,
        secret: &str,
    ) -> Result<OpenSubsonicClient, String>;
}

#[derive(Debug, Clone)]
pub struct ConnectionService {
    client_name: String,
}

impl ConnectionService {
    pub fn new(client_name: impl Into<String>) -> Self {
        Self {
            client_name: client_name.into(),
        }
    }
}

#[async_trait]
impl ConnectionApi for ConnectionService {
    async fn connect(
        &self,
        server_url: &str,
        auth: &AuthInput,
    ) -> Result<ConnectedServer, ConnectFailure> {
        match auth {
            AuthInput::Password { username, password } => {
                if username.trim().is_empty() || password.is_empty() {
                    return Err(ConnectFailure::Auth {
                        message: "Username and credentials are required.".to_string(),
                        code: None,
                        help_url: None,
                    });
                }
            }
            AuthInput::ApiKey { api_key } => {
                if api_key.is_empty() {
                    return Err(ConnectFailure::Auth {
                        message: "API key is required.".to_string(),
                        code: None,
                        help_url: None,
                    });
                }
            }
        }

        let base_url = normalize_base_url(server_url).map_err(map_api_error)?;
        let probe = ServerProbe::new(self.client_name.clone()).map_err(map_api_error)?;
        let features = probe
            .fetch_server_features(&base_url)
            .await
            .map_err(map_api_error)?;

        if matches!(auth, AuthInput::ApiKey { .. }) && !features.has_extension("apiKeyAuthentication")
        {
            return Err(ConnectFailure::UnsupportedAuth {
                message: "This server does not advertise OpenSubsonic API key authentication."
                    .to_string(),
            });
        }

        let client = OpenSubsonicClient::new(ClientConfig::new(
            base_url.clone(),
            to_client_auth(auth),
            self.client_name.clone(),
        ))
        .map_err(map_api_error)?;

        let ping = client.ping().await.map_err(map_api_error)?;
        let capability_matrix = map_capabilities(&features, ping.meta.open_subsonic.unwrap_or(false));
        let resolved_username = match auth {
            AuthInput::Password { username, .. } => username.trim().to_string(),
            AuthInput::ApiKey { .. } => client
                .token_info()
                .await
                .map_err(map_api_error)?
                .payload
                .token_info
                .username,
        };

        Ok(ConnectedServer {
            normalized_server_url: base_url.to_string(),
            username: resolved_username,
            auth_kind: auth.auth_kind(),
            api_version: ping.meta.api_version,
            server_type: ping.meta.server_type,
            server_version: ping.meta.server_version,
            capability_matrix,
        })
    }

    fn build_client_from_active_session(
        &self,
        session: &ActiveSession,
        secret: &str,
    ) -> Result<OpenSubsonicClient, String> {
        let base_url = normalize_base_url(&session.normalized_server_url)
            .map_err(|error| format_connection_error(&error))?;
        let auth = match session.auth_kind {
            AuthKind::Password => Auth::Token {
                username: session.username.clone(),
                password: secret.to_string(),
            },
            AuthKind::ApiKey => Auth::ApiKey {
                api_key: secret.to_string(),
            },
        };

        OpenSubsonicClient::new(ClientConfig::new(base_url, auth, self.client_name.clone()))
            .map_err(|error| format_connection_error(&error))
    }
}

fn to_client_auth(auth: &AuthInput) -> Auth {
    match auth {
        AuthInput::Password { username, password } => Auth::Token {
            username: username.trim().to_string(),
            password: password.to_string(),
        },
        AuthInput::ApiKey { api_key } => Auth::ApiKey {
            api_key: api_key.to_string(),
        },
    }
}

fn map_capabilities(features: &ServerFeatures, ping_open_subsonic: bool) -> CapabilityMatrix {
    CapabilityMatrix::from_extensions(
        features.open_subsonic || ping_open_subsonic,
        features
            .raw_extensions
            .iter()
            .map(|extension| OpenSubsonicExtension {
                name: extension.name.clone(),
                versions: extension.versions.clone(),
            })
            .collect(),
    )
}

fn map_api_error(error: ApiError) -> ConnectFailure {
    match error {
        ApiError::InvalidUrl(message) | ApiError::ClientBuild(message) => {
            ConnectFailure::Network { message }
        }
        ApiError::Transport(error) => ConnectFailure::Network {
            message: describe_transport_error(error),
        },
        ApiError::HttpStatus {
            status,
            body_preview,
        } if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) => {
            ConnectFailure::Auth {
                message: format_http_status_message(status, body_preview.as_deref()),
                code: None,
                help_url: None,
            }
        }
        ApiError::HttpStatus {
            status,
            body_preview,
        } => ConnectFailure::Server {
            message: format_http_status_message(status, body_preview.as_deref()),
            code: None,
            help_url: None,
        },
        ApiError::Decode {
            message,
            body_preview,
        } => ConnectFailure::Server {
            message: format_decode_message(&message, body_preview.as_deref()),
            code: None,
            help_url: None,
        },
        ApiError::Api {
            code,
            message,
            help_url,
            ..
        } => match code {
            40 | 44 => ConnectFailure::Auth {
                message: message.unwrap_or_else(|| "The server rejected the credentials.".to_string()),
                code: Some(code),
                help_url,
            },
            41 | 42 => ConnectFailure::UnsupportedAuth {
                message: message
                    .unwrap_or_else(|| "The selected authentication mechanism is not supported.".to_string()),
            },
            43 => ConnectFailure::Server {
                message: message.unwrap_or_else(|| {
                    "The server reported conflicting authentication parameters.".to_string()
                }),
                code: Some(code),
                help_url,
            },
            _ => ConnectFailure::Server {
                message: message.unwrap_or_else(|| "The server rejected the request.".to_string()),
                code: Some(code),
                help_url,
            },
        },
        ApiError::UnsupportedExtension { extension } => ConnectFailure::Server {
            message: format!("The server is missing the required OpenSubsonic extension: {extension}."),
            code: None,
            help_url: None,
        },
        ApiError::Protocol(message) => ConnectFailure::Server {
            message,
            code: None,
            help_url: None,
        },
    }
}

fn describe_transport_error(error: reqwest::Error) -> String {
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

fn format_http_status_message(status: StatusCode, body_preview: Option<&str>) -> String {
    let mut message = format!(
        "The server returned HTTP {} during the connection handshake.",
        status.as_u16()
    );
    if let Some(body_preview) = body_preview {
        if !body_preview.is_empty() {
            message.push_str(" Response preview: ");
            message.push_str(body_preview);
        }
    }
    message
}

fn format_decode_message(message: &str, body_preview: Option<&str>) -> String {
    let mut formatted =
        format!("The server returned a response that could not be decoded: {message}");
    if let Some(body_preview) = body_preview {
        if !body_preview.is_empty() {
            formatted.push_str(" Response preview: ");
            formatted.push_str(body_preview);
        }
    }
    formatted
}

#[allow(dead_code)]
fn format_connection_error(error: &ApiError) -> String {
    match error {
        ApiError::InvalidUrl(message)
        | ApiError::ClientBuild(message)
        | ApiError::Protocol(message) => message.clone(),
        ApiError::Transport(error) => error.to_string(),
        ApiError::HttpStatus { status, .. } => format!("HTTP {}", status.as_u16()),
        ApiError::Decode { message, .. } => message.clone(),
        ApiError::Api {
            code, message, ..
        } => {
            let detail = message.clone().unwrap_or_else(|| "Unknown API error".to_string());
            format!("API {code}: {detail}")
        }
        ApiError::UnsupportedExtension { extension } => {
            format!("Missing extension: {extension}")
        }
    }
}

#[cfg(test)]
mod tests {
    use mockito::{Matcher, Server};

    use super::{ConnectFailure, ConnectionApi, ConnectionService};
    use crate::models::AuthInput;

    #[tokio::test]
    async fn api_key_connect_resolves_username_via_token_info() {
        let mut server = Server::new_async().await;

        server
            .mock("GET", "/rest/getOpenSubsonicExtensions.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("v".into(), "1.13.0".into()),
                Matcher::UrlEncoded("c".into(), "transonic".into()),
                Matcher::UrlEncoded("f".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                  "subsonic-response": {
                    "status": "ok",
                    "version": "1.16.1",
                    "openSubsonic": true,
                    "openSubsonicExtensions": [
                      { "name": "apiKeyAuthentication", "versions": [1] }
                    ]
                  }
                }"#,
            )
            .create_async()
            .await;

        server
            .mock("GET", "/rest/ping.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("apiKey".into(), "secret-key".into()),
                Matcher::UrlEncoded("v".into(), "1.13.0".into()),
                Matcher::UrlEncoded("c".into(), "transonic".into()),
                Matcher::UrlEncoded("f".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                  "subsonic-response": {
                    "status": "ok",
                    "version": "1.16.1",
                    "type": "Navidrome",
                    "serverVersion": "0.54.0",
                    "openSubsonic": true
                  }
                }"#,
            )
            .create_async()
            .await;

        server
            .mock("GET", "/rest/tokenInfo.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("apiKey".into(), "secret-key".into()),
                Matcher::UrlEncoded("v".into(), "1.13.0".into()),
                Matcher::UrlEncoded("c".into(), "transonic".into()),
                Matcher::UrlEncoded("f".into(), "json".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                  "subsonic-response": {
                    "status": "ok",
                    "version": "1.16.1",
                    "openSubsonic": true,
                    "tokenInfo": { "username": "resolved-user" }
                  }
                }"#,
            )
            .create_async()
            .await;

        let service = ConnectionService::new("transonic");
        let connected = service
            .connect(
                &server.url(),
                &AuthInput::ApiKey {
                    api_key: "secret-key".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(connected.username, "resolved-user");
        assert!(connected.capability_matrix.api_key_auth);
        assert!(connected.capability_matrix.open_subsonic);
    }

    #[tokio::test]
    async fn api_key_connect_rejects_server_without_extension() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/rest/getOpenSubsonicExtensions.view")
            .with_status(404)
            .create_async()
            .await;

        let service = ConnectionService::new("transonic");
        let result = service
            .connect(
                &server.url(),
                &AuthInput::ApiKey {
                    api_key: "secret-key".to_string(),
                },
            )
            .await;

        match result {
            Err(ConnectFailure::UnsupportedAuth { .. }) => {}
            other => panic!("expected unsupported auth, got {other:?}"),
        }
    }
}
