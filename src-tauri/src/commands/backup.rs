use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::{
    models::{
        AppSettings, ServerBackupExportResult, ServerBackupImportRequest, ServerBackupImportResult,
    },
    profiles::{
        load_profiles_file, metadata_path, save_profiles_file, ProfilesFile, StoredProfileMetadata,
    },
    secrets::{create_secret_store, SecretStore},
    ActiveSessionState, AppSettingsState,
};

const BACKUP_FILENAME: &str = "transonic-backup.json";
const BACKUP_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerBackupFile {
    version: u32,
    exported_at: u64,
    app_version: String,
    settings: AppSettings,
    profiles: Vec<ServerBackupProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerBackupProfile {
    metadata: StoredProfileMetadata,
    secret: String,
}

#[tauri::command]
#[specta::specta]
pub fn export_server_backup(
    app: AppHandle,
    settings: State<'_, AppSettingsState>,
) -> Result<ServerBackupExportResult, String> {
    let config_dir = config_dir(&app)?;
    let profiles = load_profiles_file(&metadata_path(&config_dir))?;
    let secret_store = create_secret_store(&config_dir);
    let secret_service_name = secret_service_name(&app);

    let backup_profiles = profiles
        .profiles
        .into_iter()
        .map(|metadata| {
            let secret = secret_store
                .load_secret(&secret_service_name, &metadata.profile_id)?
                .ok_or_else(|| {
                    format!(
                        "Stored credentials are missing for profile {}.",
                        metadata.display_name
                    )
                })?;
            Ok(ServerBackupProfile { metadata, secret })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let app_settings = settings
        .0
        .lock()
        .map_err(|_| "The app settings state is unavailable.".to_string())?
        .snapshot()
        .0;
    let profile_count = u32::try_from(backup_profiles.len()).unwrap_or(u32::MAX);
    let backup = ServerBackupFile {
        version: BACKUP_VERSION,
        exported_at: current_timestamp_millis(),
        app_version: app.package_info().version.to_string(),
        settings: app_settings,
        profiles: backup_profiles,
    };

    let path = write_backup_to_first_available_path(&app, &backup)?;
    Ok(ServerBackupExportResult {
        path: path.to_string_lossy().to_string(),
        profile_count,
    })
}

#[tauri::command]
#[specta::specta]
pub fn import_server_backup(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    settings: State<'_, AppSettingsState>,
    payload: ServerBackupImportRequest,
) -> Result<ServerBackupImportResult, String> {
    let (path, backup) = read_backup_from_fixed_paths(&app)?;
    let profile_count = import_backup_file(&app, &settings, backup, payload.replace_existing)?;
    *state
        .0
        .lock()
        .map_err(|_| "The active session state is unavailable.".to_string())? = None;
    crate::connect::restart(&app);
    Ok(ServerBackupImportResult {
        path: path.to_string_lossy().to_string(),
        profile_count,
    })
}

pub(crate) fn maybe_auto_import_server_backup(
    app: &AppHandle,
    settings: &State<'_, AppSettingsState>,
) -> Result<bool, String> {
    let config_dir = config_dir(app)?;
    let existing_profiles = load_profiles_file(&metadata_path(&config_dir))?;
    if !existing_profiles.profiles.is_empty() {
        return Ok(false);
    }

    let Ok((_path, backup)) = read_backup_from_fixed_paths(app) else {
        return Ok(false);
    };
    import_backup_file(app, settings, backup, false)?;
    Ok(true)
}

fn import_backup_file(
    app: &AppHandle,
    settings: &State<'_, AppSettingsState>,
    backup: ServerBackupFile,
    replace_existing: bool,
) -> Result<u32, String> {
    if backup.version != BACKUP_VERSION {
        return Err(format!("Unsupported backup version {}.", backup.version));
    }

    let config_dir = config_dir(app)?;
    let profiles_path = metadata_path(&config_dir);
    let existing_profiles = load_profiles_file(&profiles_path)?;
    if !replace_existing && !existing_profiles.profiles.is_empty() {
        return Err(
            "Server profiles already exist. Import with replaceExisting to overwrite them."
                .to_string(),
        );
    }

    let secret_store = create_secret_store(&config_dir);
    let secret_service_name = secret_service_name(app);
    for profile in &backup.profiles {
        secret_store.save_secret(
            &secret_service_name,
            &profile.metadata.profile_id,
            &profile.secret,
        )?;
    }

    let profiles = ProfilesFile {
        version: BACKUP_VERSION,
        profiles: backup
            .profiles
            .into_iter()
            .map(|profile| profile.metadata)
            .collect(),
    };
    let profile_count = u32::try_from(profiles.profiles.len()).unwrap_or(u32::MAX);
    save_profiles_file(&profiles_path, &profiles)?;

    settings
        .0
        .lock()
        .map_err(|_| "The app settings state is unavailable.".to_string())?
        .replace(backup.settings)?;

    Ok(profile_count)
}

fn write_backup_to_first_available_path(
    app: &AppHandle,
    backup: &ServerBackupFile,
) -> Result<PathBuf, String> {
    let mut last_error = None;
    for path in fixed_backup_paths(app) {
        match write_backup_file(&path, backup) {
            Ok(()) => return Ok(path),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| "No backup path is available.".to_string()))
}

fn read_backup_from_fixed_paths(app: &AppHandle) -> Result<(PathBuf, ServerBackupFile), String> {
    let mut last_error = None;
    for path in fixed_backup_paths(app) {
        match fs::read_to_string(&path) {
            Ok(contents) => {
                let backup = serde_json::from_str::<ServerBackupFile>(&contents)
                    .map_err(|error| format!("Failed to parse server backup: {error}"))?;
                return Ok((path, backup));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => last_error = Some(format!("Failed to read server backup: {error}")),
        }
    }

    Err(last_error.unwrap_or_else(|| "No server backup file was found.".to_string()))
}

fn write_backup_file(path: &Path, backup: &ServerBackupFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create backup directory: {error}"))?;
    }

    let json = serde_json::to_string_pretty(backup)
        .map_err(|error| format!("Failed to serialize server backup: {error}"))?;
    let mut file = fs::File::create(path)
        .map_err(|error| format!("Failed to open server backup for writing: {error}"))?;
    file.write_all(json.as_bytes())
        .map_err(|error| format!("Failed to write server backup: {error}"))
}

fn fixed_backup_paths(app: &AppHandle) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(download_dir) = app.path().download_dir() {
        paths.push(download_dir.join(BACKUP_FILENAME));
    }
    if let Ok(config_dir) = app.path().app_config_dir() {
        let config_path = config_dir.join(BACKUP_FILENAME);
        if !paths.iter().any(|path| path == &config_path) {
            paths.push(config_path);
        }
    }
    paths
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app config directory: {error}"))
}

fn secret_service_name(app: &AppHandle) -> String {
    format!("{}.server-profile", app.config().identifier)
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
