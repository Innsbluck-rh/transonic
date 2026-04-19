mod app_settings;
mod artist_image_cache;
pub mod bindings;
mod commands;
mod connection;
mod cover_art_cache;
mod models;
mod playback;
mod playback_state;
mod profiles;
mod secrets;
mod session;
use std::sync::{Arc, Mutex};

use tauri::Manager;

use app_settings::{app_settings_path, AppSettingsStore};
use artist_image_cache::ArtistImageCache;
use cover_art_cache::CoverArtCache;
use models::ActiveSession;
use playback::{PlaybackController, PlaybackEventAppHandle};
use playback_state::{playback_state_path, FilePlaybackStatePersister};

pub(crate) struct ActiveSessionState(pub Mutex<Option<ActiveSession>>);
pub(crate) struct PlaybackControllerState(pub Mutex<PlaybackController>);
pub(crate) struct AppSettingsState(pub Mutex<AppSettingsStore>);
pub(crate) struct CoverArtCacheState(pub CoverArtCache);
pub(crate) struct ArtistImageCacheState(pub ArtistImageCache);

impl Default for ActiveSessionState {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

impl PlaybackControllerState {
    fn new(controller: PlaybackController) -> Self {
        Self(Mutex::new(controller))
    }
}

impl AppSettingsState {
    fn new(store: AppSettingsStore) -> Self {
        Self(Mutex::new(store))
    }
}

impl CoverArtCacheState {
    fn new(cache: CoverArtCache) -> Self {
        Self(cache)
    }
}

impl ArtistImageCacheState {
    fn new(cache: ArtistImageCache) -> Self {
        Self(cache)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = bindings::builder();

    let builder = tauri::Builder::default()
        .manage(ActiveSessionState::default())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("transonic_lib::commands::playback", log::LevelFilter::Trace)
                .level_for("transonic_lib::playback", log::LevelFilter::Trace)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .invoke_handler(specta_builder.invoke_handler());

    #[cfg(target_os = "android")]
    let builder = builder.plugin(playback::init_android_mobile_plugin());

    let app = builder
        .setup(move |app| {
            specta_builder.mount_events(app);

            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|error| format!("Failed to resolve the app config directory: {error}"))?;
            let settings_path = app_settings_path(&config_dir);
            let settings_store = match AppSettingsStore::load(settings_path.clone()) {
                Ok(store) => store,
                Err(error) => {
                    log::warn!("setup: failed to load app settings, using defaults: {error}");
                    AppSettingsStore::default_at(settings_path)
                }
            };
            let initial_gapless_playback_enabled = settings_store.snapshot().0.playback.gapless_playback_enabled;
            app.manage(AppSettingsState::new(settings_store));
            let persister_path = playback_state_path(&config_dir);
            let persister = Box::new(FilePlaybackStatePersister::new(persister_path));

            let app_handle: PlaybackEventAppHandle =
                Arc::new(Mutex::new(Some(app.handle().clone())));
            let controller = playback::create_playback_controller(
                &app.handle(),
                app_handle.clone(),
                persister,
                initial_gapless_playback_enabled,
            );
            app.manage(PlaybackControllerState::new(controller));
            let cover_art_cache_root = app
                .path()
                .app_cache_dir()
                .map_err(|error| {
                    format!("Failed to resolve the app cache directory for cover art: {error}")
                })?
                .join("cover-art");
            app.manage(CoverArtCacheState::new(CoverArtCache::new(
                cover_art_cache_root,
            )));
            let artist_image_cache_root = app
                .path()
                .app_cache_dir()
                .map_err(|error| {
                    format!("Failed to resolve the app cache directory for artist images: {error}")
                })?
                .join("artist-image");
            app.manage(ArtistImageCacheState::new(ArtistImageCache::new(
                artist_image_cache_root,
            )));

            #[cfg(target_os = "android")]
            playback::install_android_app_handle(app.handle().clone());
            #[cfg(target_os = "windows")]
            playback::install_windows_app_handle(app.handle().clone());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            let playback = app_handle.state::<PlaybackControllerState>();
            let mut controller = match playback.0.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            controller.sync_and_persist();
            log::info!("app exit: persisted playback state");
        }
    });
}
