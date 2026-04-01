use crate::models::PlaybackStatus;

pub trait PlaybackReporter: Send {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct NoopPlaybackReporter;

impl PlaybackReporter for NoopPlaybackReporter {
    fn report_state(&mut self, _status: &PlaybackStatus) -> Result<(), String> {
        Ok(())
    }
}
