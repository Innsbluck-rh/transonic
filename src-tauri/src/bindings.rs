use std::path::PathBuf;

use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

use crate::commands;

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::bootstrap_app_state,
        commands::connect_server_profile,
        commands::delete_server_profile,
        commands::get_music_folders,
        commands::get_folder_structure_roots,
        commands::get_folder_structure_albums,
        commands::get_artist_indexes,
        commands::get_music_directory,
        commands::get_album_list,
        commands::get_cover_art
    ])
}

pub fn export_typescript_bindings() -> Result<(), specta_typescript::ExportError> {
    builder().export(Typescript::default(), bindings_path())
}

pub fn bindings_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("bindings.ts")
}
