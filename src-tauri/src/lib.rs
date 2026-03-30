pub mod bindings;
mod browse_parser;
mod commands;
mod connection;
mod folder_structure;
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

    #[cfg(debug_assertions)]
    bindings::export_typescript_bindings().expect("failed to export TypeScript bindings");

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
