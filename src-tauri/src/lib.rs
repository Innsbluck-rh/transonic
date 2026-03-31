pub mod bindings;
mod commands;
mod connection;
mod models;
mod profiles;
mod secrets;
mod session;

use std::sync::Mutex;

use models::ActiveSession;

pub(crate) struct ActiveSessionState(pub Mutex<Option<ActiveSession>>);

impl Default for ActiveSessionState {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = bindings::builder();

    tauri::Builder::default()
        .manage(ActiveSessionState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
