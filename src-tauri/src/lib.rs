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
    tauri::Builder::default()
        .manage(ActiveSessionState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_app_state,
            commands::connect_server_profile,
            commands::delete_server_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
