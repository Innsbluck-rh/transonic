pub mod bindings;
mod commands;
mod connection;
mod models;
mod playback;
mod profiles;
mod secrets;
mod session;
use std::sync::{Arc, Mutex};

use tauri::Manager;

use models::ActiveSession;
use playback::{PlaybackController, PlaybackEventAppHandle};

pub(crate) struct ActiveSessionState(pub Mutex<Option<ActiveSession>>);
pub(crate) struct PlaybackControllerState(
    pub Mutex<PlaybackController>,
    pub PlaybackEventAppHandle,
);

impl Default for ActiveSessionState {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

impl Default for PlaybackControllerState {
    fn default() -> Self {
        let app_handle: PlaybackEventAppHandle = Arc::new(Mutex::new(None));
        Self(
            Mutex::new(PlaybackController::with_tauri_reporter(app_handle.clone())),
            app_handle,
        )
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = bindings::builder();

    tauri::Builder::default()
        .manage(ActiveSessionState::default())
        .manage(PlaybackControllerState::default())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("transonic_lib::commands::playback", log::LevelFilter::Trace)
                .level_for("transonic_lib::playback", log::LevelFilter::Trace)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            let playback = app.state::<PlaybackControllerState>();
            *playback.1.lock().unwrap() = Some(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
