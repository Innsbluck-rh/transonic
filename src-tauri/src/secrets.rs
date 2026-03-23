#[cfg(test)]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub trait SecretStore: Send + Sync {
    fn save_secret(&self, service_name: &str, profile_id: &str, secret: &str)
        -> Result<(), String>;

    fn load_secret(&self, service_name: &str, profile_id: &str) -> Result<Option<String>, String>;

    fn delete_secret(&self, service_name: &str, profile_id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OsKeyringSecretStore;

impl SecretStore for OsKeyringSecretStore {
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

#[cfg(test)]
fn build_key(service_name: &str, profile_id: &str) -> String {
    format!("{service_name}:{profile_id}")
}
