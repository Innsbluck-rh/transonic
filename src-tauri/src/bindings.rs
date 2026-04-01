use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use specta_typescript::Typescript;
use tauri_specta::{collect_commands, collect_events, Builder};

use crate::{commands, models::PlaybackStatus};

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::bootstrap_app_state,
            commands::connect_server_profile,
            commands::delete_server_profile,
            commands::get_music_folders,
            commands::get_folder_structure_roots,
            commands::get_folder_structure_albums,
            commands::get_folder_structure_album_songs,
            commands::get_artist_indexes,
            commands::get_music_directory,
            commands::get_album_songs,
            commands::get_song,
            commands::get_album_list,
            commands::get_cover_art,
            commands::playback_get_state,
            commands::playback_set_queue,
            commands::playback_play,
            commands::playback_pause,
            commands::playback_stop,
            commands::playback_seek,
            commands::playback_next,
            commands::playback_prev
        ])
        .events(collect_events![PlaybackStatus])
}

pub fn export_typescript_bindings() -> Result<(), specta_typescript::ExportError> {
    export_typescript_bindings_to(&bindings_path())
}

pub fn check_typescript_bindings() -> Result<(), BindingsCheckError> {
    let current_bindings_path = bindings_path();
    let generated_bindings = generate_typescript_bindings()?;
    let current_bindings =
        fs::read_to_string(&current_bindings_path).map_err(BindingsCheckError::Io)?;

    if normalize_line_endings(&generated_bindings) != normalize_line_endings(&current_bindings) {
        return Err(BindingsCheckError::Drift {
            path: current_bindings_path,
        });
    }

    Ok(())
}

pub fn bindings_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("bindings.ts")
}

fn export_typescript_bindings_to(path: &Path) -> Result<(), specta_typescript::ExportError> {
    builder().export(Typescript::default(), path.to_path_buf())
}

fn generate_typescript_bindings() -> Result<String, BindingsCheckError> {
    let temp_bindings_path = temporary_bindings_path();
    let _temp_bindings_file = TemporaryBindingsFile::new(temp_bindings_path.clone());

    export_typescript_bindings_to(&temp_bindings_path).map_err(BindingsCheckError::Export)?;

    fs::read_to_string(temp_bindings_path).map_err(BindingsCheckError::Io)
}

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

fn temporary_bindings_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!(
        "transonic-bindings-{}-{}.ts",
        std::process::id(),
        timestamp
    ))
}

struct TemporaryBindingsFile {
    path: PathBuf,
}

impl TemporaryBindingsFile {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TemporaryBindingsFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Debug)]
pub enum BindingsCheckError {
    Export(specta_typescript::ExportError),
    Io(io::Error),
    Drift { path: PathBuf },
}

impl fmt::Display for BindingsCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Export(err) => write!(f, "failed to generate TypeScript bindings: {err}"),
            Self::Io(err) => write!(f, "failed to read TypeScript bindings: {err}"),
            Self::Drift { path } => write!(
                f,
                "TypeScript bindings are stale: {}. Run `pnpm bindings:export`.",
                path.display()
            ),
        }
    }
}

impl Error for BindingsCheckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Export(..) => None,
            Self::Io(err) => Some(err),
            Self::Drift { .. } => None,
        }
    }
}
