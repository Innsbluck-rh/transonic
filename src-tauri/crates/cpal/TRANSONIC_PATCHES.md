# Transonic CPAL Patch

This vendored CPAL 0.18.1 copy contains one Transonic-specific WASAPI output patch:

- `src/host/wasapi/device.rs` skips the pre-`Initialize` `IsFormatSupported` rejection for render endpoints.

Reason:

- CPAL 0.18's `DeviceHandle::DefaultOutput` is required for Windows System Default stream rerouting.
- Querying `default_output_config()` before stream creation can fail after COM was initialized with a different apartment mode on the same thread.
- WASAPI output streams already pass `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` to `IAudioClient::Initialize`, and CPAL's own source comment says output streams can rely on this conversion.
- Letting `Initialize` decide keeps default stream rerouting while avoiding false `Stream configuration is not supported in shared mode` failures from the preflight check.
