use opensubsonic_client::PreparedBinaryRequest;

pub trait PlaybackBackend: Send {
    fn load(
        &mut self,
        request: PreparedBinaryRequest,
        start_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String>;
    fn seek(&mut self, position_ms: u32) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}
