use std::sync::{Arc, Mutex};

use tauri_specta::Event;

use crate::models::PlaybackStatus;

pub type PlaybackEventAppHandle = Arc<Mutex<Option<tauri::AppHandle>>>;

pub trait PlaybackReporter: Send {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String>;
}

#[derive(Clone)]
pub struct TauriPlaybackReporter {
    app_handle: PlaybackEventAppHandle,
}

impl TauriPlaybackReporter {
    pub fn new(app_handle: PlaybackEventAppHandle) -> Self {
        Self { app_handle }
    }
}

impl PlaybackReporter for TauriPlaybackReporter {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
        let handle = self.app_handle.lock().unwrap();
        let Some(app_handle) = handle.as_ref() else {
            return Ok(());
        };

        status.emit(app_handle).map_err(|error| error.to_string())
    }
}
