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
pub(crate) struct PlaybackControllerState(pub Mutex<PlaybackController>);

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

    builder
        .setup(move |app| {
            specta_builder.mount_events(app);

            let app_handle: PlaybackEventAppHandle =
                Arc::new(Mutex::new(Some(app.handle().clone())));
            let controller =
                playback::create_playback_controller(&app.handle(), app_handle.clone());
            app.manage(PlaybackControllerState::new(controller));

            #[cfg(target_os = "android")]
            playback::install_android_app_handle(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
