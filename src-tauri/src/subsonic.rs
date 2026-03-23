use std::time::Duration;

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

const API_VERSION: &str = "1.13.0";
const CLIENT_NAME: &str = "transonic";
const REQUEST_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSubsonicConnectionRequest {
    pub server_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConnectionCheckResult {
    Success {
        #[serde(rename = "normalizedServerUrl")]
        normalized_server_url: String,
        username: String,
        #[serde(rename = "apiVersion")]
        api_version: String,
        #[serde(rename = "serverType")]
        server_type: Option<String>,
        #[serde(rename = "serverVersion")]
        server_version: Option<String>,
        #[serde(rename = "openSubsonic")]
        open_subsonic: Option<bool>,
    },
    AuthError {
        message: String,
        code: Option<u32>,
        #[serde(rename = "helpUrl")]
        help_url: Option<String>,
    },
    NetworkError {
        message: String,
    },
    ServerError {
        message: String,
        code: Option<u32>,
        #[serde(rename = "helpUrl")]
        help_url: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct SubsonicEnvelope {
    #[serde(rename = "subsonic-response")]
    response: SubsonicResponse,
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
struct SubsonicError {
    code: u32,
    message: Option<String>,
    #[serde(rename = "helpUrl")]
    help_url: Option<String>,
}

enum ErrorCategory {
    Auth,
    Server,
}

pub async fn check_subsonic_connection_impl(
    payload: CheckSubsonicConnectionRequest,
) -> ConnectionCheckResult {
    let normalized_server_url = match normalize_server_url(&payload.server_url) {
        Ok(url) => url,
        Err(message) => return ConnectionCheckResult::NetworkError { message },
    };

    let username = payload.username.trim().to_owned();
    if username.is_empty() || payload.password.is_empty() {
        return ConnectionCheckResult::AuthError {
            message: "Username and password are required.".to_string(),
            code: None,
            help_url: None,
        };
    }

    let client = match Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return ConnectionCheckResult::NetworkError {
                message: format!("Failed to prepare the HTTP client: {error}"),
            };
        }
    };

    // ping.view is the smallest authenticated connectivity check in Subsonic.
    let ping_url = format!("{normalized_server_url}/ping.view");
    let salt = generate_salt();
    // Token authentication avoids sending the raw password on every request.
    let token = build_token(&payload.password, &salt);

    let response = match client
        .get(&ping_url)
        .query(&[
            ("u", username.as_str()),
            ("t", token.as_str()),
            ("s", salt.as_str()),
            ("v", API_VERSION),
            ("c", CLIENT_NAME),
            ("f", "json"),
        ])
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return ConnectionCheckResult::NetworkError {
                message: describe_network_error(error),
            };
        }
    };

    let http_status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            return ConnectionCheckResult::NetworkError {
                message: format!("Failed to read the server response: {error}"),
            };
        }
    };

    match serde_json::from_str::<SubsonicEnvelope>(&body) {
        Ok(envelope) => {
            classify_subsonic_response(envelope.response, normalized_server_url, username)
        }
        Err(_)
            if matches!(
                http_status,
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ) =>
        {
            ConnectionCheckResult::AuthError {
                message:
                    "The server rejected the credentials before returning a Subsonic response."
                        .to_string(),
                code: None,
                help_url: None,
            }
        }
        Err(_) => ConnectionCheckResult::ServerError {
            message: format!(
                "The server returned HTTP {} without a valid Subsonic JSON response.",
                http_status.as_u16()
            ),
            code: None,
            help_url: None,
        },
    }
}

fn normalize_server_url(server_url: &str) -> Result<String, String> {
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

    // Many servers expose a web UI at the root, but the REST API always lives under /rest.
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

fn generate_salt() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

fn build_token(password: &str, salt: &str) -> String {
    format!("{:x}", md5::compute(format!("{password}{salt}")))
}

fn classify_subsonic_response(
    response: SubsonicResponse,
    normalized_server_url: String,
    username: String,
) -> ConnectionCheckResult {
    if response.status.eq_ignore_ascii_case("ok") {
        return ConnectionCheckResult::Success {
            normalized_server_url,
            username,
            api_version: response.version,
            server_type: response.server_type,
            server_version: response.server_version,
            open_subsonic: response.open_subsonic,
        };
    }

    let Some(error) = response.error else {
        return ConnectionCheckResult::ServerError {
            message: "The server reported a failure without an error payload.".to_string(),
            code: None,
            help_url: None,
        };
    };

    let message = error
        .message
        .unwrap_or_else(|| "The server rejected the request.".to_string());

    // OpenSubsonic extends the original list, but these codes still describe auth failures.
    match classify_error_code(error.code) {
        ErrorCategory::Auth => ConnectionCheckResult::AuthError {
            message,
            code: Some(error.code),
            help_url: error.help_url,
        },
        ErrorCategory::Server => ConnectionCheckResult::ServerError {
            message,
            code: Some(error.code),
            help_url: error.help_url,
        },
    }
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
    use super::{build_token, classify_error_code, normalize_server_url, ErrorCategory};

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
    fn classify_authentication_errors_as_auth_failures() {
        assert!(matches!(classify_error_code(40), ErrorCategory::Auth));
        assert!(matches!(classify_error_code(42), ErrorCategory::Auth));
        assert!(matches!(classify_error_code(44), ErrorCategory::Auth));
    }

    #[test]
    fn classify_other_subsonic_errors_as_server_failures() {
        assert!(matches!(classify_error_code(20), ErrorCategory::Server));
        assert!(matches!(classify_error_code(50), ErrorCategory::Server));
        assert!(matches!(classify_error_code(70), ErrorCategory::Server));
    }
}
