#[cfg(target_os = "android")]
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackNativeEvent {
    Buffering { position_ms: u32 },
    Ready { position_ms: u32 },
    Playing { position_ms: u32 },
    Paused { position_ms: u32 },
    SeekProcessed { position_ms: u32 },
    Error { message: String },
    Ended,
}

pub trait NativePlaybackEventSource: Send {
    fn drain_events(&mut self) -> Vec<PlaybackNativeEvent>;
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct NoopNativePlaybackEventSource;

impl NativePlaybackEventSource for NoopNativePlaybackEventSource {
    fn drain_events(&mut self) -> Vec<PlaybackNativeEvent> {
        Vec::new()
    }
}

#[cfg(target_os = "android")]
#[derive(Debug, Clone, Default)]
pub struct AndroidPlaybackEventHub {
    queue: Arc<Mutex<VecDeque<PlaybackNativeEvent>>>,
}

#[cfg(target_os = "android")]
impl AndroidPlaybackEventHub {
    pub fn push(&self, event: PlaybackNativeEvent) {
        self.queue.lock().unwrap().push_back(event);
    }
}

#[cfg(target_os = "android")]
impl NativePlaybackEventSource for AndroidPlaybackEventHub {
    fn drain_events(&mut self) -> Vec<PlaybackNativeEvent> {
        let mut queue = self.queue.lock().unwrap();
        queue.drain(..).collect()
    }
}
