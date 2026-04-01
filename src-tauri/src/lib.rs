pub mod bindings;
mod commands;
mod connection;
mod models;
mod playback;
mod profiles;
mod secrets;
mod session;

use std::sync::Mutex;

use models::ActiveSession;
use playback::PlaybackController;

pub(crate) struct ActiveSessionState(pub Mutex<Option<ActiveSession>>);
pub(crate) struct PlaybackControllerState(pub Mutex<PlaybackController>);

impl Default for ActiveSessionState {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

impl Default for PlaybackControllerState {
    fn default() -> Self {
        Self(Mutex::new(PlaybackController::with_defaults()))
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
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
