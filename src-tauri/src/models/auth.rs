use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    Password,
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthInput {
    Password {
        username: String,
        password: String,
    },
    ApiKey {
        #[serde(rename = "apiKey")]
        api_key: String,
    },
}

impl AuthInput {
    pub fn auth_kind(&self) -> AuthKind {
        match self {
            Self::Password { .. } => AuthKind::Password,
            Self::ApiKey { .. } => AuthKind::ApiKey,
        }
    }

    pub fn secret(&self) -> &str {
        match self {
            Self::Password { password, .. } => password,
            Self::ApiKey { api_key } => api_key,
        }
    }

    pub fn restored(auth_kind: AuthKind, username: String, secret: String) -> Self {
        match auth_kind {
            AuthKind::Password => Self::Password {
                username,
                password: secret,
            },
            AuthKind::ApiKey => Self::ApiKey { api_key: secret },
        }
    }
}
