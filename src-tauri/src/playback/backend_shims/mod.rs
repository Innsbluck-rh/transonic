mod backend;
#[allow(unused_imports)]
pub use backend::{
    PlaybackBackend, PlaybackBackendLoadRequest, PlaybackLoadStrategy, PlaybackSeekAction,
};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "android")]
mod android;

#[cfg(not(any(target_os = "windows", target_os = "android")))]
mod unsupported;

#[cfg(target_os = "windows")]
pub use windows::create_playback_backend;

#[cfg(target_os = "android")]
pub use android::create_playback_backend;

#[cfg(not(any(target_os = "windows", target_os = "android")))]
pub use unsupported::create_playback_backend;
