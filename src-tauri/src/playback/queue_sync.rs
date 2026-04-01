use crate::models::PlaybackStatus;

pub trait QueueSyncGateway: Send {
    fn sync_queue(&mut self, status: &PlaybackStatus) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct NoopQueueSyncGateway;

impl QueueSyncGateway for NoopQueueSyncGateway {
    fn sync_queue(&mut self, _status: &PlaybackStatus) -> Result<(), String> {
        Ok(())
    }
}
