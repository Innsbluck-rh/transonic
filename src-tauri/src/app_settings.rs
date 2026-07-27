use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::{
    normalize_output_device_id, normalize_transcoding_bitrate_limit, normalize_volume, AppSettings,
    AppearanceSettings, ConnectSettings, PlaybackSettings, SettingsOrigin,
};

const APP_SETTINGS_FILENAME: &str = "app-settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    appearance: AppearanceSettings,
    #[serde(default)]
    playback: PlaybackSettings,
    #[serde(default)]
    connect: ConnectSettings,
}

impl Default for AppSettingsFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            appearance: AppearanceSettings::default(),
            playback: PlaybackSettings::default(),
            connect: ConnectSettings::default(),
        }
    }
}

impl From<AppSettingsFile> for AppSettings {
    fn from(value: AppSettingsFile) -> Self {
        Self {
            appearance: value.appearance,
            playback: value.playback,
            connect: value.connect,
        }
    }
}

impl From<AppSettings> for AppSettingsFile {
    fn from(value: AppSettings) -> Self {
        Self {
            version: default_version(),
            appearance: value.appearance,
            playback: value.playback,
            connect: value.connect,
        }
    }
}

#[derive(Debug)]
pub struct AppSettingsStore {
    path: PathBuf,
    settings: AppSettings,
    origin: SettingsOrigin,
}

impl AppSettingsStore {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        match load_app_settings_file(&path)? {
            Some(file) => Ok(Self {
                path,
                settings: file.into(),
                origin: SettingsOrigin::Stored,
            }),
            None => Ok(Self::default_at(path)),
        }
    }

    pub fn default_at(path: PathBuf) -> Self {
        Self {
            path,
            settings: AppSettings::default(),
            origin: SettingsOrigin::Default,
        }
    }

    pub fn snapshot(&self) -> (AppSettings, SettingsOrigin) {
        (self.settings.clone(), self.origin.clone())
    }

    pub fn replace(&mut self, settings: AppSettings) -> Result<AppSettings, String> {
        let settings = normalize_app_settings(settings);
        save_app_settings_file(&self.path, &settings)?;
        self.settings = settings.clone();
        self.origin = SettingsOrigin::Stored;
        Ok(settings)
    }
}

pub fn app_settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join(APP_SETTINGS_FILENAME)
}

fn load_app_settings_file(path: &Path) -> Result<Option<AppSettingsFile>, String> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let file: AppSettingsFile = serde_json::from_str(&contents)
                .map_err(|error| format!("Failed to parse app settings: {error}"))?;
            if file.version != default_version() {
                return Err(format!(
                    "Unsupported app settings version {}.",
                    file.version
                ));
            }
            Ok(Some(file))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Failed to read app settings: {error}")),
    }
}

fn save_app_settings_file(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create the settings directory: {error}"))?;
    }

    let json = serde_json::to_string_pretty(&AppSettingsFile::from(settings.clone()))
        .map_err(|error| format!("Failed to serialize app settings: {error}"))?;

    let mut file = fs::File::create(path)
        .map_err(|error| format!("Failed to open the app settings file for writing: {error}"))?;
    file.write_all(json.as_bytes())
        .map_err(|error| format!("Failed to write the app settings file: {error}"))?;
    Ok(())
}

fn normalize_app_settings(mut settings: AppSettings) -> AppSettings {
    settings.playback.volume = normalize_volume(settings.playback.volume);
    settings.playback.output_device_id =
        normalize_output_device_id(settings.playback.output_device_id);
    settings.playback.transcoding_bitrate_limit =
        normalize_transcoding_bitrate_limit(settings.playback.transcoding_bitrate_limit);
    settings
}

fn default_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{app_settings_path, AppSettingsStore};
    use crate::models::{
        AlbumDisplayMode, AppSettings, PlaybackStreamMode, PlaybackTranscodingCodec,
    };

    #[test]
    fn missing_settings_file_uses_defaults() {
        let dir = tempdir().unwrap();
        let store = AppSettingsStore::load(app_settings_path(dir.path())).unwrap();
        let (settings, origin) = store.snapshot();

        assert_eq!(settings, AppSettings::default());
        assert_eq!(origin, crate::models::SettingsOrigin::Default);
    }

    #[test]
    fn settings_store_persists_updates() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        let mut store = AppSettingsStore::load(path.clone()).unwrap();
        let mut settings = AppSettings::default();
        settings.appearance.album_display_mode = AlbumDisplayMode::List;
        settings.playback.gapless_playback_enabled = true;
        settings.playback.volume = 0.42;
        settings.playback.use_custom_output = true;
        settings.playback.output_device_id = Some("output:device-1".to_string());
        settings.playback.stream_mode = PlaybackStreamMode::Transcoding;
        settings.playback.metered_network_transcoding_enabled = true;
        settings.playback.transcoding_bitrate_limit = 192;
        settings.playback.use_custom_transcoding_codec = true;
        settings.playback.transcoding_codec = PlaybackTranscodingCodec::Opus;

        store.replace(settings.clone()).unwrap();

        let reloaded = AppSettingsStore::load(path).unwrap();
        let (restored_settings, origin) = reloaded.snapshot();
        assert_eq!(restored_settings, settings);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn settings_store_clamps_playback_volume_when_replacing() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        let mut store = AppSettingsStore::load(path).unwrap();
        let mut settings = AppSettings::default();
        settings.playback.volume = 5.0;

        let stored = store.replace(settings).unwrap();

        assert_eq!(stored.playback.volume, 1.0);
    }

    #[test]
    fn settings_store_normalizes_zero_transcoding_bitrate_limit_when_replacing() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        let mut store = AppSettingsStore::load(path).unwrap();
        let mut settings = AppSettings::default();
        settings.playback.transcoding_bitrate_limit = 0;

        let stored = store.replace(settings).unwrap();

        assert_eq!(stored.playback.transcoding_bitrate_limit, 320);
    }

    #[test]
    fn settings_store_normalizes_empty_output_device_id_when_replacing() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        let mut store = AppSettingsStore::load(path).unwrap();
        let mut settings = AppSettings::default();
        settings.playback.output_device_id = Some("  ".to_string());

        let stored = store.replace(settings).unwrap();

        assert_eq!(stored.playback.output_device_id, None);
    }

    #[test]
    fn playback_volume_defaults_and_clamps_when_loading_settings() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"volume\": 2.0\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(settings.playback.volume, 1.0);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn playback_stream_mode_and_transcoding_bitrate_limit_load_settings() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"streamMode\": \"transcoding\",\n    \"transcodingBitrateLimit\": 256\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(
            settings.playback.stream_mode,
            PlaybackStreamMode::Transcoding
        );
        assert_eq!(settings.playback.transcoding_bitrate_limit, 256);
        assert!(!settings.playback.metered_network_transcoding_enabled);
        assert!(!settings.playback.use_custom_transcoding_codec);
        assert_eq!(
            settings.playback.transcoding_codec,
            PlaybackTranscodingCodec::Auto
        );
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn playback_custom_transcoding_codec_loads_settings() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"useCustomTranscodingCodec\": true,\n    \"transcodingCodec\": \"opus\"\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert!(settings.playback.use_custom_transcoding_codec);
        assert_eq!(
            settings.playback.transcoding_codec,
            PlaybackTranscodingCodec::Opus
        );
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn missing_playback_stream_settings_use_defaults() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"gaplessPlaybackEnabled\": true\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(settings.playback.stream_mode, PlaybackStreamMode::Raw);
        assert_eq!(settings.playback.transcoding_bitrate_limit, 320);
        assert!(!settings.playback.metered_network_transcoding_enabled);
        assert!(!settings.playback.use_custom_transcoding_codec);
        assert_eq!(
            settings.playback.transcoding_codec,
            PlaybackTranscodingCodec::Auto
        );
        assert_eq!(settings.playback.output_device_id, None);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn playback_output_device_id_loads_settings() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"outputDeviceId\": \"output:device-1\"\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert!(settings.playback.use_custom_output);
        assert_eq!(
            settings.playback.output_device_id.as_deref(),
            Some("output:device-1")
        );
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn playback_use_asio_output_defaults_to_false_for_settings_without_it() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"outputDeviceId\": \"output:device-1\"\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, _) = store.snapshot();
        assert!(!settings.playback.use_asio_output);
    }

    #[test]
    fn playback_use_asio_output_loads_from_settings() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"useAsioOutput\": true\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert!(settings.playback.use_asio_output);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn empty_playback_output_device_id_loads_as_none() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"outputDeviceId\": \"   \"\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(settings.playback.output_device_id, None);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn legacy_gapless_setting_key_is_migrated() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"prebufferEnabled\": true\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert!(settings.playback.gapless_playback_enabled);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn legacy_gapless_strategy_is_migrated() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"prebufferStrategy\": \"next_track\"\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert!(settings.playback.gapless_playback_enabled);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn missing_connect_settings_use_defaults() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"gaplessPlaybackEnabled\": true\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(settings.connect, crate::models::ConnectSettings::default());
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn missing_appearance_settings_use_defaults() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"playback\": {\n    \"gaplessPlaybackEnabled\": true\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(
            settings.appearance,
            crate::models::AppearanceSettings::default()
        );
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }

    #[test]
    fn legacy_connect_server_url_is_split_into_host_and_port() {
        let dir = tempdir().unwrap();
        let path = app_settings_path(dir.path());
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"connect\": {\n    \"serverUrl\": \"http://100.86.122.91:4747\",\n    \"connectServerPort\": 9999\n  }\n}\n",
        )
        .unwrap();

        let store = AppSettingsStore::load(path).unwrap();
        let (settings, origin) = store.snapshot();
        assert_eq!(
            settings.connect.connect_server_host.as_deref(),
            Some("http://100.86.122.91")
        );
        assert_eq!(settings.connect.connect_server_port, 4747);
        assert_eq!(origin, crate::models::SettingsOrigin::Stored);
    }
}
