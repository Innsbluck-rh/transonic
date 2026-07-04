#[cfg(not(test))]
use std::sync::{Arc, Mutex};

#[cfg(not(test))]
use tauri_specta::Event;

use crate::models::PlaybackStatus;

#[cfg(not(test))]
pub type PlaybackEventAppHandle = Arc<Mutex<Option<tauri::AppHandle>>>;

pub trait PlaybackReporter: Send {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String>;
}

pub struct CompositePlaybackReporter {
    reporters: Vec<Box<dyn PlaybackReporter>>,
}

impl CompositePlaybackReporter {
    pub fn new(reporters: Vec<Box<dyn PlaybackReporter>>) -> Self {
        Self { reporters }
    }
}

impl PlaybackReporter for CompositePlaybackReporter {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
        let mut last_error = None;
        for reporter in &mut self.reporters {
            if let Err(error) = reporter.report_state(status) {
                last_error = Some(error);
            }
        }

        if let Some(error) = last_error {
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(not(test))]
#[derive(Clone)]
pub struct TauriPlaybackReporter {
    app_handle: PlaybackEventAppHandle,
}

#[cfg(not(test))]
impl TauriPlaybackReporter {
    pub fn new(app_handle: PlaybackEventAppHandle) -> Self {
        Self { app_handle }
    }
}

#[cfg(not(test))]
impl PlaybackReporter for TauriPlaybackReporter {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
        let handle = self.app_handle.lock().unwrap();
        let Some(app_handle) = handle.as_ref() else {
            return Ok(());
        };

        status.emit(app_handle).map_err(|error| error.to_string())
    }
}
