use std::{
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::{
    connection::{ConnectedServer, ConnectionApi},
    models::{
        ActiveSession, AppBootstrap, AppSettings, AuthInput, ConnectServerProfileRequest,
        ConnectServerProfileResult, LastConnectionState, PlaybackCapabilities, RestoreStatus,
        SettingsOrigin,
    },
    profiles::{
        load_profiles_file, metadata_path, save_profiles_file, server_url_components, ProfilesFile,
        StoredProfileMetadata, StoredServerInfo,
    },
    secrets::SecretStore,
};

use opensubsonic_client::OpenSubsonicClient;

pub struct SessionService<S, A> {
    metadata_path: PathBuf,
    secret_service_name: String,
    secret_store: S,
    api: A,
}

impl<S, A> SessionService<S, A>
where
    S: SecretStore,
    A: ConnectionApi,
{
    pub fn new(config_dir: PathBuf, secret_service_name: String, secret_store: S, api: A) -> Self {
        Self {
            metadata_path: metadata_path(&config_dir),
            secret_service_name,
            secret_store,
            api,
        }
    }

    pub async fn bootstrap_app_state(
        &self,
        active_session_state: &Mutex<Option<ActiveSession>>,
    ) -> Result<AppBootstrap, String> {
        let mut profiles = self.load_profiles()?;
        let current_active = current_active_session(active_session_state);
        if current_active.is_some() {
            return Ok(bootstrap_response(
                profiles.summaries(),
                current_active,
                RestoreStatus::None,
                None,
            ));
        }

        let Some(active_profile_id) = profiles
            .profiles
            .iter()
            .find(|profile| profile.is_last_active)
            .map(|profile| profile.profile_id.clone())
        else {
            return Ok(bootstrap_response(
                profiles.summaries(),
                None,
                RestoreStatus::None,
                None,
            ));
        };

        let Some(profile) = profiles.find_profile(&active_profile_id).cloned() else {
            return Ok(bootstrap_response(
                profiles.summaries(),
                None,
                RestoreStatus::None,
                None,
            ));
        };

        let Some(secret) = self
            .secret_store
            .load_secret(&self.secret_service_name, &profile.profile_id)?
        else {
            update_profile_failure(
                &mut profiles,
                &profile.profile_id,
                LastConnectionState::ReauthRequired,
                true,
            );
            self.save_profiles(&profiles)?;
            set_active_session(active_session_state, None);
            return Ok(bootstrap_response(
                profiles.summaries(),
                None,
                RestoreStatus::ConnectionError,
                Some(
                    "Stored credentials are missing. Re-enter the credentials for the active profile."
                        .to_string(),
                ),
            ));
        };

        let auth = AuthInput::restored(profile.auth_kind.clone(), profile.username.clone(), secret);
        match self
            .api
            .connect(&profile.normalized_server_url, &auth)
            .await
        {
            Ok(connection) => {
                let active_session = store_connected_profile(
                    &mut profiles,
                    profile.profile_id.clone(),
                    Some(profile.display_name),
                    connection,
                );
                self.save_profiles(&profiles)?;
                set_active_session(active_session_state, Some(active_session.clone()));

                Ok(bootstrap_response(
                    profiles.summaries(),
                    Some(active_session),
                    RestoreStatus::Restored,
                    Some("Restored the last active server profile.".to_string()),
                ))
            }
            Err(failure) => {
                update_profile_failure(
                    &mut profiles,
                    &profile.profile_id,
                    failure.last_connection_state(),
                    true,
                );
                self.save_profiles(&profiles)?;
                set_active_session(active_session_state, None);
                Ok(bootstrap_response(
                    profiles.summaries(),
                    None,
                    failure.restore_status(),
                    Some(failure.message().to_string()),
                ))
            }
        }
    }

    pub async fn connect_server_profile(
        &self,
        active_session_state: &Mutex<Option<ActiveSession>>,
        payload: ConnectServerProfileRequest,
    ) -> Result<ConnectServerProfileResult, String> {
        let mut profiles = self.load_profiles()?;
        let existing_profile = match payload.profile_id.as_deref() {
            Some(profile_id) => profiles.find_profile(profile_id).cloned(),
            None => None,
        };

        if payload.profile_id.is_some() && existing_profile.is_none() {
            return Err("The selected server profile no longer exists.".to_string());
        }

        match self.api.connect(&payload.server_url, &payload.auth).await {
            Ok(connection) => {
                if has_duplicate_profile_identity(
                    &profiles,
                    payload.profile_id.as_deref(),
                    &connection.normalized_server_url,
                    &connection.username,
                ) {
                    return Err(
                        "A server profile for this server and username already exists.".to_string(),
                    );
                }

                let profile_id = payload
                    .profile_id
                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                let display_name = choose_display_name(
                    payload.display_name,
                    existing_profile
                        .as_ref()
                        .map(|profile| profile.display_name.as_str()),
                    &connection.normalized_server_url,
                );
                let active_session = store_connected_profile(
                    &mut profiles,
                    profile_id.clone(),
                    Some(display_name),
                    connection,
                );
                self.secret_store.save_secret(
                    &self.secret_service_name,
                    &profile_id,
                    payload.auth.secret(),
                )?;
                self.save_profiles(&profiles)?;
                set_active_session(active_session_state, Some(active_session.clone()));

                Ok(ConnectServerProfileResult::Connected {
                    active_session,
                    profiles: profiles.summaries(),
                })
            }
            Err(failure) => {
                if let Some(profile_id) = payload.profile_id.as_deref() {
                    update_profile_failure(
                        &mut profiles,
                        profile_id,
                        failure.last_connection_state(),
                        false,
                    );
                    self.save_profiles(&profiles)?;
                }

                Ok(failure.into_public_result(
                    profiles.summaries(),
                    current_active_session(active_session_state),
                ))
            }
        }
    }

    pub fn delete_server_profile(
        &self,
        active_session_state: &Mutex<Option<ActiveSession>>,
        profile_id: &str,
    ) -> Result<AppBootstrap, String> {
        let mut profiles = self.load_profiles()?;
        let deleted_active_profile = current_active_session(active_session_state)
            .map(|session| session.profile_id == profile_id)
            .unwrap_or(false);

        profiles.remove_profile(profile_id);
        self.secret_store
            .delete_secret(&self.secret_service_name, profile_id)?;
        self.save_profiles(&profiles)?;

        if deleted_active_profile {
            set_active_session(active_session_state, None);
        }

        Ok(bootstrap_response(
            profiles.summaries(),
            if deleted_active_profile {
                None
            } else {
                current_active_session(active_session_state)
            },
            RestoreStatus::None,
            Some("Deleted the selected server profile.".to_string()),
        ))
    }

    #[allow(dead_code)]
    pub fn build_active_client(
        &self,
        active_session_state: &Mutex<Option<ActiveSession>>,
    ) -> Result<OpenSubsonicClient, String> {
        let Some(session) = current_active_session(active_session_state) else {
            return Err("No active session is available.".to_string());
        };

        let Some(secret) = self
            .secret_store
            .load_secret(&self.secret_service_name, &session.profile_id)?
        else {
            return Err(
                "Stored credentials are missing. Re-enter the credentials for the active profile."
                    .to_string(),
            );
        };

        self.api.build_client_from_active_session(&session, &secret)
    }

    pub fn active_auth_input(
        &self,
        active_session_state: &Mutex<Option<ActiveSession>>,
    ) -> Result<(ActiveSession, AuthInput), String> {
        let Some(session) = current_active_session(active_session_state) else {
            return Err("No active session is available.".to_string());
        };

        let Some(secret) = self
            .secret_store
            .load_secret(&self.secret_service_name, &session.profile_id)?
        else {
            return Err(
                "Stored credentials are missing. Re-enter the credentials for the active profile."
                    .to_string(),
            );
        };

        let auth = AuthInput::restored(session.auth_kind.clone(), session.username.clone(), secret);
        Ok((session, auth))
    }

    fn load_profiles(&self) -> Result<ProfilesFile, String> {
        load_profiles_file(&self.metadata_path)
    }

    fn save_profiles(&self, profiles: &ProfilesFile) -> Result<(), String> {
        save_profiles_file(&self.metadata_path, profiles)
    }
}

fn store_connected_profile(
    profiles: &mut ProfilesFile,
    profile_id: String,
    display_name: Option<String>,
    connection: ConnectedServer,
) -> ActiveSession {
    let display_name = display_name.unwrap_or_else(|| connection.normalized_server_url.clone());
    let active_session = connection.clone().into_active_session(profile_id.clone());
    let url_components = server_url_components(&active_session.normalized_server_url);

    profiles.set_last_active(Some(&profile_id));
    profiles.upsert_profile(StoredProfileMetadata {
        profile_id: profile_id.clone(),
        display_name,
        normalized_server_url: active_session.normalized_server_url.clone(),
        server_url_host: url_components.host,
        server_url_port: url_components.port,
        server_url_path: url_components.path,
        auth_kind: active_session.auth_kind.clone(),
        username: active_session.username.clone(),
        last_connection_state: LastConnectionState::Ok,
        last_capability_snapshot: Some(active_session.capability_matrix.clone()),
        last_server_info: Some(StoredServerInfo {
            api_version: active_session.api_version.clone(),
            server_type: active_session.server_type.clone(),
            server_version: active_session.server_version.clone(),
        }),
        last_connected_at: Some(current_timestamp_millis()),
        is_last_active: true,
    });

    active_session
}

fn update_profile_failure(
    profiles: &mut ProfilesFile,
    profile_id: &str,
    state: LastConnectionState,
    mark_last_active: bool,
) {
    if mark_last_active {
        profiles.set_last_active(Some(profile_id));
    }

    if let Some(profile) = profiles.find_profile_mut(profile_id) {
        profile.last_connection_state = state;
    }
}

fn choose_display_name(
    requested_display_name: Option<String>,
    existing_display_name: Option<&str>,
    fallback_url: &str,
) -> String {
    if let Some(display_name) = requested_display_name {
        let trimmed = display_name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(existing_display_name) = existing_display_name {
        if !existing_display_name.trim().is_empty() {
            return existing_display_name.to_string();
        }
    }

    fallback_url.to_string()
}

fn has_duplicate_profile_identity(
    profiles: &ProfilesFile,
    current_profile_id: Option<&str>,
    normalized_server_url: &str,
    username: &str,
) -> bool {
    profiles.profiles.iter().any(|profile| {
        profile.normalized_server_url == normalized_server_url
            && profile.username == username
            && Some(profile.profile_id.as_str()) != current_profile_id
    })
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn bootstrap_response(
    profiles: Vec<crate::models::SavedProfileSummary>,
    active_session: Option<ActiveSession>,
    restore_status: RestoreStatus,
    message: Option<String>,
) -> AppBootstrap {
    AppBootstrap {
        profiles,
        active_session,
        restore_status,
        message,
        playback_capabilities: PlaybackCapabilities {
            gapless_playback: false,
        },
        settings: AppSettings::default(),
        settings_origin: SettingsOrigin::Default,
    }
}

fn current_active_session(
    active_session_state: &Mutex<Option<ActiveSession>>,
) -> Option<ActiveSession> {
    active_session_state.lock().unwrap().clone()
}

fn set_active_session(
    active_session_state: &Mutex<Option<ActiveSession>>,
    session: Option<ActiveSession>,
) {
    *active_session_state.lock().unwrap() = session;
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use tempfile::tempdir;

    use super::SessionService;
    use crate::{
        connection::{ConnectFailure, ConnectedServer, ConnectionApi},
        models::{
            ActiveSession, AuthInput, AuthKind, CapabilityMatrix, ConnectServerProfileRequest,
            ConnectServerProfileResult, LastConnectionState, RestoreStatus,
        },
        secrets::{InMemorySecretStore, SecretStore},
    };

    use opensubsonic_client::OpenSubsonicClient;

    #[derive(Clone, Default)]
    struct MockSubsonicApi {
        responses: Arc<Mutex<VecDeque<Result<ConnectedServer, ConnectFailure>>>>,
    }

    impl MockSubsonicApi {
        fn push_response(&self, response: Result<ConnectedServer, ConnectFailure>) {
            self.responses.lock().unwrap().push_back(response);
        }
    }

    #[async_trait]
    impl ConnectionApi for MockSubsonicApi {
        async fn connect(
            &self,
            _server_url: &str,
            _auth: &AuthInput,
        ) -> Result<ConnectedServer, ConnectFailure> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("mock response missing")
        }

        fn build_client_from_active_session(
            &self,
            session: &ActiveSession,
            secret: &str,
        ) -> Result<OpenSubsonicClient, String> {
            opensubsonic_client::OpenSubsonicClient::new(opensubsonic_client::ClientConfig::new(
                opensubsonic_client::normalize_base_url(&session.normalized_server_url)
                    .map_err(|error| format!("{error:?}"))?,
                match session.auth_kind {
                    AuthKind::Password => opensubsonic_client::Auth::Token {
                        username: session.username.clone(),
                        password: secret.to_string(),
                    },
                    AuthKind::ApiKey => opensubsonic_client::Auth::ApiKey {
                        api_key: secret.to_string(),
                    },
                },
                "transonic",
            ))
            .map_err(|error| format!("{error:?}"))
        }
    }

    fn connected_server() -> ConnectedServer {
        ConnectedServer {
            normalized_server_url: "https://demo.example/rest".to_string(),
            username: "demo".to_string(),
            auth_kind: AuthKind::Password,
            api_version: "1.16.1".to_string(),
            server_type: Some("Navidrome".to_string()),
            server_version: Some("0.52.0".to_string()),
            capability_matrix: CapabilityMatrix::empty(),
        }
    }

    #[tokio::test]
    async fn connect_persists_profiles_and_credentials() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let api = MockSubsonicApi::default();
        api.push_response(Ok(connected_server()));
        let service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            api,
        );
        let active_session_state = Mutex::new(None);

        let result = service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        match result {
            ConnectServerProfileResult::Connected {
                active_session,
                profiles,
            } => {
                assert_eq!(profiles.len(), 1);
                assert_eq!(active_session.username, "demo");
                assert!(secret_store
                    .load_secret("transonic", &active_session.profile_id)
                    .unwrap()
                    .is_some());
            }
            other => panic!("expected connected result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn connect_accepts_api_key_profiles_without_username_input() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let api = MockSubsonicApi::default();
        api.push_response(Ok(ConnectedServer {
            normalized_server_url: "https://demo.example/rest".to_string(),
            username: "resolved-api-user".to_string(),
            auth_kind: AuthKind::ApiKey,
            api_version: "1.16.1".to_string(),
            server_type: Some("Navidrome".to_string()),
            server_version: Some("0.54.0".to_string()),
            capability_matrix: CapabilityMatrix::empty(),
        }));
        let service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            api,
        );
        let active_session_state = Mutex::new(None);

        let result = service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("API Key Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::ApiKey {
                        api_key: "secret-key".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        match result {
            ConnectServerProfileResult::Connected { active_session, .. } => {
                assert_eq!(active_session.username, "resolved-api-user");
                assert_eq!(active_session.auth_kind, AuthKind::ApiKey);
                assert!(secret_store
                    .load_secret("transonic", &active_session.profile_id)
                    .unwrap()
                    .is_some());
            }
            other => panic!("expected connected result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn bootstrap_returns_none_when_no_profiles_exist() {
        let temp_dir = tempdir().unwrap();
        let service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            InMemorySecretStore::default(),
            MockSubsonicApi::default(),
        );
        let active_session_state = Mutex::new(None);

        let bootstrap = service
            .bootstrap_app_state(&active_session_state)
            .await
            .unwrap();

        assert!(bootstrap.profiles.is_empty());
        assert!(bootstrap.active_session.is_none());
        assert!(matches!(bootstrap.restore_status, RestoreStatus::None));
    }

    #[tokio::test]
    async fn bootstrap_restores_the_last_active_profile() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let connect_api = MockSubsonicApi::default();
        connect_api.push_response(Ok(connected_server()));
        let connect_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            connect_api,
        );
        let active_session_state = Mutex::new(None);

        let connect_result = connect_service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let profile_id = match connect_result {
            ConnectServerProfileResult::Connected { active_session, .. } => {
                active_session.profile_id
            }
            _ => unreachable!(),
        };

        let restore_api = MockSubsonicApi::default();
        restore_api.push_response(Ok(connected_server()));
        let restore_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            restore_api,
        );
        let active_session_state = Mutex::new(None);
        let bootstrap = restore_service
            .bootstrap_app_state(&active_session_state)
            .await
            .unwrap();

        assert!(matches!(bootstrap.restore_status, RestoreStatus::Restored));
        assert_eq!(bootstrap.active_session.unwrap().profile_id, profile_id);
    }

    #[tokio::test]
    async fn bootstrap_returns_network_error_when_reconnect_fails_with_network_error() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let connect_api = MockSubsonicApi::default();
        connect_api.push_response(Ok(connected_server()));
        let connect_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            connect_api,
        );
        let active_session_state = Mutex::new(None);

        connect_service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let restore_api = MockSubsonicApi::default();
        restore_api.push_response(Err(ConnectFailure::Network {
            message: "offline".to_string(),
            is_no_network: true,
        }));
        let restore_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            restore_api,
        );
        let active_session_state = Mutex::new(None);
        let bootstrap = restore_service
            .bootstrap_app_state(&active_session_state)
            .await
            .unwrap();

        assert!(matches!(
            bootstrap.restore_status,
            RestoreStatus::NetworkError
        ));
        assert!(bootstrap.active_session.is_none());
        assert_eq!(
            bootstrap.profiles[0].last_connection_state,
            LastConnectionState::Offline
        );
    }

    #[tokio::test]
    async fn bootstrap_returns_connection_error_when_reconnect_fails_with_server_error() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let connect_api = MockSubsonicApi::default();
        connect_api.push_response(Ok(connected_server()));
        let connect_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            connect_api,
        );
        let active_session_state = Mutex::new(None);

        connect_service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let restore_api = MockSubsonicApi::default();
        restore_api.push_response(Err(ConnectFailure::Server {
            message: "server exploded".to_string(),
            code: None,
            help_url: None,
        }));
        let restore_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            restore_api,
        );
        let active_session_state = Mutex::new(None);
        let bootstrap = restore_service
            .bootstrap_app_state(&active_session_state)
            .await
            .unwrap();

        assert!(matches!(
            bootstrap.restore_status,
            RestoreStatus::ConnectionError
        ));
        assert!(bootstrap.active_session.is_none());
        assert_eq!(
            bootstrap.profiles[0].last_connection_state,
            LastConnectionState::Offline
        );
    }

    #[tokio::test]
    async fn bootstrap_returns_connection_error_when_secret_is_missing() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let connect_api = MockSubsonicApi::default();
        connect_api.push_response(Ok(connected_server()));
        let connect_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            connect_api,
        );
        let active_session_state = Mutex::new(None);

        let result = connect_service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let profile_id = match result {
            ConnectServerProfileResult::Connected { active_session, .. } => {
                active_session.profile_id
            }
            _ => unreachable!(),
        };
        secret_store
            .delete_secret("transonic", &profile_id)
            .unwrap();

        let restore_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            MockSubsonicApi::default(),
        );
        let active_session_state = Mutex::new(None);
        let bootstrap = restore_service
            .bootstrap_app_state(&active_session_state)
            .await
            .unwrap();

        assert!(matches!(
            bootstrap.restore_status,
            RestoreStatus::ConnectionError
        ));
        assert_eq!(
            bootstrap.profiles[0].last_connection_state,
            LastConnectionState::ReauthRequired
        );
    }

    #[tokio::test]
    async fn bootstrap_returns_connection_error_for_non_network_transport_failures() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let connect_api = MockSubsonicApi::default();
        connect_api.push_response(Ok(connected_server()));
        let connect_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            connect_api,
        );
        let active_session_state = Mutex::new(None);

        connect_service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let restore_api = MockSubsonicApi::default();
        restore_api.push_response(Err(ConnectFailure::Network {
            message: "The connection attempt timed out.".to_string(),
            is_no_network: false,
        }));
        let restore_service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            restore_api,
        );
        let active_session_state = Mutex::new(None);
        let bootstrap = restore_service
            .bootstrap_app_state(&active_session_state)
            .await
            .unwrap();

        assert!(matches!(
            bootstrap.restore_status,
            RestoreStatus::ConnectionError
        ));
        assert_eq!(
            bootstrap.profiles[0].last_connection_state,
            LastConnectionState::Offline
        );
    }

    #[tokio::test]
    async fn delete_server_profile_removes_metadata_and_secret() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let api = MockSubsonicApi::default();
        api.push_response(Ok(connected_server()));
        let service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store.clone(),
            api,
        );
        let active_session_state = Mutex::new(None);

        let result = service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let profile_id = match result {
            ConnectServerProfileResult::Connected { active_session, .. } => {
                active_session.profile_id
            }
            _ => unreachable!(),
        };

        let bootstrap = service
            .delete_server_profile(&active_session_state, &profile_id)
            .unwrap();

        assert!(bootstrap.profiles.is_empty());
        assert!(secret_store
            .load_secret("transonic", &profile_id)
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn connect_rejects_duplicate_server_and_username_profiles() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let api = MockSubsonicApi::default();
        api.push_response(Ok(connected_server()));
        api.push_response(Ok(connected_server()));
        let service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            api,
        );
        let active_session_state = Mutex::new(None);

        service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        let error = service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo Duplicate".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap_err();

        assert_eq!(
            error,
            "A server profile for this server and username already exists."
        );
        assert_eq!(service.load_profiles().unwrap().profiles.len(), 1);
    }

    #[tokio::test]
    async fn build_active_client_uses_the_active_session_secret() {
        let temp_dir = tempdir().unwrap();
        let secret_store = InMemorySecretStore::default();
        let api = MockSubsonicApi::default();
        api.push_response(Ok(connected_server()));
        let service = SessionService::new(
            temp_dir.path().to_path_buf(),
            "transonic".to_string(),
            secret_store,
            api,
        );
        let active_session_state = Mutex::new(None);

        let result = service
            .connect_server_profile(
                &active_session_state,
                ConnectServerProfileRequest {
                    profile_id: None,
                    display_name: Some("Demo".to_string()),
                    server_url: "https://demo.example".to_string(),
                    auth: AuthInput::Password {
                        username: "demo".to_string(),
                        password: "sesame".to_string(),
                    },
                },
            )
            .await
            .unwrap();

        match result {
            ConnectServerProfileResult::Connected { .. } => {
                let client = service.build_active_client(&active_session_state).unwrap();
                assert_eq!(
                    client.config().base_url.as_str(),
                    "https://demo.example/rest"
                );
            }
            other => panic!("expected connected result, got {other:?}"),
        }
    }
}
