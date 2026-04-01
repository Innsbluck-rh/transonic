use std::path::Path;

#[cfg(any(test, target_os = "android"))]
use std::{
    collections::HashMap,
    fs,
    io::{ErrorKind, Write},
    path::PathBuf,
};

#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(any(test, target_os = "android"))]
use serde::{Deserialize, Serialize};

pub trait SecretStore: Send + Sync {
    fn save_secret(&self, service_name: &str, profile_id: &str, secret: &str)
        -> Result<(), String>;

    fn load_secret(&self, service_name: &str, profile_id: &str) -> Result<Option<String>, String>;

    fn delete_secret(&self, service_name: &str, profile_id: &str) -> Result<(), String>;
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct AppSecretStore;

#[cfg(not(target_os = "android"))]
pub fn create_secret_store(_config_dir: &Path) -> AppSecretStore {
    AppSecretStore
}

#[cfg(not(target_os = "android"))]
impl SecretStore for AppSecretStore {
    fn save_secret(
        &self,
        service_name: &str,
        profile_id: &str,
        secret: &str,
    ) -> Result<(), String> {
        let entry = keyring::Entry::new(service_name, profile_id)
            .map_err(|error| format!("Failed to open the OS keyring entry: {error}"))?;
        entry
            .set_password(secret)
            .map_err(|error| format!("Failed to save credentials in the OS keyring: {error}"))
    }

    fn load_secret(&self, service_name: &str, profile_id: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(service_name, profile_id)
            .map_err(|error| format!("Failed to open the OS keyring entry: {error}"))?;

        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!(
                "Failed to load credentials from the OS keyring: {error}"
            )),
        }
    }

    fn delete_secret(&self, service_name: &str, profile_id: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(service_name, profile_id)
            .map_err(|error| format!("Failed to open the OS keyring entry: {error}"))?;

        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!(
                "Failed to delete credentials from the OS keyring: {error}"
            )),
        }
    }
}

#[cfg(target_os = "android")]
pub type AppSecretStore = FileSecretStore;

#[cfg(target_os = "android")]
pub fn create_secret_store(config_dir: &Path) -> AppSecretStore {
    // Temporary fallback until Android secure-store integration is wired in.
    FileSecretStore::new(config_dir.join(ANDROID_SECRETS_FILENAME))
}

#[cfg(any(test, target_os = "android"))]
const ANDROID_SECRETS_FILENAME: &str = "secrets-v1.json";

#[cfg(any(test, target_os = "android"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileSecrets {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    entries: HashMap<String, String>,
}

#[cfg(any(test, target_os = "android"))]
impl Default for FileSecrets {
    fn default() -> Self {
        Self {
            version: default_version(),
            entries: HashMap::new(),
        }
    }
}

#[cfg(any(test, target_os = "android"))]
#[derive(Debug, Clone)]
pub struct FileSecretStore {
    secrets_path: PathBuf,
}

#[cfg(any(test, target_os = "android"))]
impl FileSecretStore {
    pub fn new(secrets_path: PathBuf) -> Self {
        Self { secrets_path }
    }

    fn load_file(&self) -> Result<FileSecrets, String> {
        match fs::read_to_string(&self.secrets_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|error| format!("Failed to parse the secret store: {error}")),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(FileSecrets::default()),
            Err(error) => Err(format!("Failed to read the secret store: {error}")),
        }
    }

    fn save_file(&self, secrets: &FileSecrets) -> Result<(), String> {
        if let Some(parent) = self.secrets_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create the secret store directory: {error}"))?;
        }

        let json = serde_json::to_string_pretty(secrets)
            .map_err(|error| format!("Failed to serialize the secret store: {error}"))?;

        let mut file = fs::File::create(&self.secrets_path)
            .map_err(|error| format!("Failed to open the secret store for writing: {error}"))?;
        file.write_all(json.as_bytes())
            .map_err(|error| format!("Failed to write the secret store: {error}"))?;

        Ok(())
    }
}

#[cfg(any(test, target_os = "android"))]
impl SecretStore for FileSecretStore {
    fn save_secret(
        &self,
        service_name: &str,
        profile_id: &str,
        secret: &str,
    ) -> Result<(), String> {
        let mut secrets = self.load_file()?;
        secrets
            .entries
            .insert(build_key(service_name, profile_id), secret.to_string());
        self.save_file(&secrets)
    }

    fn load_secret(&self, service_name: &str, profile_id: &str) -> Result<Option<String>, String> {
        let secrets = self.load_file()?;
        Ok(secrets
            .entries
            .get(&build_key(service_name, profile_id))
            .cloned())
    }

    fn delete_secret(&self, service_name: &str, profile_id: &str) -> Result<(), String> {
        let mut secrets = self.load_file()?;
        secrets.entries.remove(&build_key(service_name, profile_id));
        self.save_file(&secrets)
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct InMemorySecretStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

#[cfg(test)]
impl SecretStore for InMemorySecretStore {
    fn save_secret(
        &self,
        service_name: &str,
        profile_id: &str,
        secret: &str,
    ) -> Result<(), String> {
        self.inner
            .lock()
            .unwrap()
            .insert(build_key(service_name, profile_id), secret.to_string());
        Ok(())
    }

    fn load_secret(&self, service_name: &str, profile_id: &str) -> Result<Option<String>, String> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(&build_key(service_name, profile_id))
            .cloned())
    }

    fn delete_secret(&self, service_name: &str, profile_id: &str) -> Result<(), String> {
        self.inner
            .lock()
            .unwrap()
            .remove(&build_key(service_name, profile_id));
        Ok(())
    }
}

#[cfg(any(test, target_os = "android"))]
fn build_key(service_name: &str, profile_id: &str) -> String {
    format!("{service_name}:{profile_id}")
}

#[cfg(any(test, target_os = "android"))]
fn default_version() -> u32 {
    1
}
