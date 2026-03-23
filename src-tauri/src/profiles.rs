use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::{AuthKind, CapabilityMatrix, LastConnectionState, SavedProfileSummary};

const PROFILES_FILENAME: &str = "profiles-v1.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredServerInfo {
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredProfileMetadata {
    pub profile_id: String,
    pub display_name: String,
    pub normalized_server_url: String,
    pub auth_kind: AuthKind,
    pub username: String,
    pub last_connection_state: LastConnectionState,
    pub last_capability_snapshot: Option<CapabilityMatrix>,
    pub last_server_info: Option<StoredServerInfo>,
    pub last_connected_at: Option<u64>,
    pub is_last_active: bool,
}

impl StoredProfileMetadata {
    pub fn to_summary(&self) -> SavedProfileSummary {
        SavedProfileSummary {
            profile_id: self.profile_id.clone(),
            display_name: self.display_name.clone(),
            normalized_server_url: self.normalized_server_url.clone(),
            auth_kind: self.auth_kind.clone(),
            username: self.username.clone(),
            last_connection_state: self.last_connection_state.clone(),
            is_active: self.is_last_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub profiles: Vec<StoredProfileMetadata>,
}

impl Default for ProfilesFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            profiles: Vec::new(),
        }
    }
}

impl ProfilesFile {
    pub fn summaries(&self) -> Vec<SavedProfileSummary> {
        self.profiles
            .iter()
            .map(StoredProfileMetadata::to_summary)
            .collect()
    }

    pub fn find_profile(&self, profile_id: &str) -> Option<&StoredProfileMetadata> {
        self.profiles
            .iter()
            .find(|profile| profile.profile_id == profile_id)
    }

    pub fn find_profile_mut(&mut self, profile_id: &str) -> Option<&mut StoredProfileMetadata> {
        self.profiles
            .iter_mut()
            .find(|profile| profile.profile_id == profile_id)
    }

    pub fn set_last_active(&mut self, profile_id: Option<&str>) {
        for profile in &mut self.profiles {
            profile.is_last_active = Some(profile.profile_id.as_str()) == profile_id;
        }
    }

    pub fn upsert_profile(&mut self, profile: StoredProfileMetadata) {
        match self
            .profiles
            .iter()
            .position(|current| current.profile_id == profile.profile_id)
        {
            Some(index) => self.profiles[index] = profile,
            None => self.profiles.push(profile),
        }
    }

    pub fn remove_profile(&mut self, profile_id: &str) {
        self.profiles
            .retain(|profile| profile.profile_id != profile_id);
    }
}

pub fn metadata_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PROFILES_FILENAME)
}

pub fn load_profiles_file(path: &Path) -> Result<ProfilesFile, String> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("Failed to parse saved profiles: {error}")),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(ProfilesFile::default()),
        Err(error) => Err(format!("Failed to read saved profiles: {error}")),
    }
}

pub fn save_profiles_file(path: &Path, profiles: &ProfilesFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create the profile directory: {error}"))?;
    }

    let json = serde_json::to_string_pretty(profiles)
        .map_err(|error| format!("Failed to serialize saved profiles: {error}"))?;

    let mut file = fs::File::create(path)
        .map_err(|error| format!("Failed to open the profile store for writing: {error}"))?;
    file.write_all(json.as_bytes())
        .map_err(|error| format!("Failed to write the profile store: {error}"))?;
    Ok(())
}

fn default_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::{ProfilesFile, StoredProfileMetadata};
    use crate::models::{AuthKind, LastConnectionState};

    #[test]
    fn profiles_file_serializes_and_restores_profiles() {
        let mut profiles = ProfilesFile::default();
        profiles.profiles.push(StoredProfileMetadata {
            profile_id: "profile-1".to_string(),
            display_name: "Demo".to_string(),
            normalized_server_url: "https://demo.example/rest".to_string(),
            auth_kind: AuthKind::Password,
            username: "demo".to_string(),
            last_connection_state: LastConnectionState::Ok,
            last_capability_snapshot: None,
            last_server_info: None,
            last_connected_at: Some(123),
            is_last_active: true,
        });

        let json = serde_json::to_string(&profiles).unwrap();
        let restored: ProfilesFile = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.profiles.len(), 1);
        assert_eq!(restored.profiles[0].profile_id, "profile-1");
        assert!(restored.profiles[0].is_last_active);
    }

    #[test]
    fn set_last_active_only_marks_the_selected_profile() {
        let mut profiles = ProfilesFile {
            version: 1,
            profiles: vec![
                StoredProfileMetadata {
                    profile_id: "one".to_string(),
                    display_name: "One".to_string(),
                    normalized_server_url: "https://one.example/rest".to_string(),
                    auth_kind: AuthKind::Password,
                    username: "one".to_string(),
                    last_connection_state: LastConnectionState::Never,
                    last_capability_snapshot: None,
                    last_server_info: None,
                    last_connected_at: None,
                    is_last_active: false,
                },
                StoredProfileMetadata {
                    profile_id: "two".to_string(),
                    display_name: "Two".to_string(),
                    normalized_server_url: "https://two.example/rest".to_string(),
                    auth_kind: AuthKind::ApiKey,
                    username: "two".to_string(),
                    last_connection_state: LastConnectionState::Never,
                    last_capability_snapshot: None,
                    last_server_info: None,
                    last_connected_at: None,
                    is_last_active: false,
                },
            ],
        };

        profiles.set_last_active(Some("two"));

        assert!(!profiles.profiles[0].is_last_active);
        assert!(profiles.profiles[1].is_last_active);
    }
}
