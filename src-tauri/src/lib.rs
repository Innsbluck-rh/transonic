mod subsonic;

use subsonic::{
    check_subsonic_connection_impl, CheckSubsonicConnectionRequest, ConnectionCheckResult,
};

#[tauri::command]
async fn check_subsonic_connection(
    payload: CheckSubsonicConnectionRequest,
) -> ConnectionCheckResult {
    check_subsonic_connection_impl(payload).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![check_subsonic_connection])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
