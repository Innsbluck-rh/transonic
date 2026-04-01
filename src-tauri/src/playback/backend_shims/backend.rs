use opensubsonic_client::PreparedBinaryRequest;

pub trait PlaybackBackend: Send {
    fn load(
        &mut self,
        request: PreparedBinaryRequest,
        absolute_start_position_ms: u32,
        local_start_position_ms: u32,
        autoplay: bool,
    ) -> Result<(), String>;
    fn seek(&mut self, position_ms: u32) -> Result<(), String>;
    fn current_position_ms(&self) -> Result<u32, String>;
    fn pause(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}
