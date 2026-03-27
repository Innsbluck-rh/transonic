use std::fmt::{Display, Formatter};

use reqwest::Url;

use crate::{auth::Auth, error::ApiError};

const DEFAULT_API_VERSION: &str = "1.13.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiVersion(String);

impl ApiVersion {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self(DEFAULT_API_VERSION.to_string())
    }
}

impl Display for ApiVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: Url,
    pub auth: Auth,
    pub client_name: String,
    pub api_version: ApiVersion,
}

impl ClientConfig {
    pub fn new(base_url: Url, auth: Auth, client_name: impl Into<String>) -> Self {
        Self {
            base_url,
            auth,
            client_name: client_name.into(),
            api_version: ApiVersion::default(),
        }
    }
}

pub fn normalize_base_url(server_url: &str) -> Result<Url, ApiError> {
    let trimmed = server_url.trim();
    if trimmed.is_empty() {
        return Err(ApiError::InvalidUrl("Server URL is required.".to_string()));
    }

    let mut url = Url::parse(trimmed).map_err(|_| {
        ApiError::InvalidUrl(
            "Enter a valid server URL starting with http:// or https://.".to_string(),
        )
    })?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(ApiError::InvalidUrl(
            "Server URL must start with http:// or https://.".to_string(),
        ));
    }

    if url.host_str().is_none() {
        return Err(ApiError::InvalidUrl(
            "Server URL must include a host name.".to_string(),
        ));
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
    Ok(url)
}
