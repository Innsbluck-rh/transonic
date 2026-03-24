use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    Token { username: String, password: String },
    ApiKey { api_key: String },
    LegacyPassword { username: String, password: String },
}

impl Auth {
    pub fn to_query_pairs(&self) -> Vec<(String, String)> {
        match self {
            Self::Token { username, password } => {
                let salt = generate_salt();
                vec![
                    ("u".to_string(), username.trim().to_string()),
                    ("t".to_string(), build_token(password, &salt)),
                    ("s".to_string(), salt),
                ]
            }
            Self::ApiKey { api_key } => vec![("apiKey".to_string(), api_key.to_string())],
            Self::LegacyPassword { username, password } => vec![
                ("u".to_string(), username.trim().to_string()),
                ("p".to_string(), password.to_string()),
            ],
        }
    }
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
