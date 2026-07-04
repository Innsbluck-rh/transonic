pub mod api;
pub mod auth;
pub mod client;
pub mod config;
pub mod envelope;
pub mod error;
pub mod extensions;
pub mod probe;
pub mod transport;
pub mod types;

pub use auth::Auth;
pub use client::OpenSubsonicClient;
pub use config::{normalize_base_url, ApiVersion, ClientConfig};
pub use envelope::{Envelope, ResponseMeta, ResponseStatus};
pub use error::ApiError;
pub use extensions::{OpenSubsonicExtension, ServerFeatures};
pub use probe::ServerProbe;
pub use transport::{PreparedBinaryRequest, ReqwestTransport};
pub use types::common::{
    AlbumListType, Child, ClientInfo, EmptyPayload, MediaType, PlaybackState,
};

#[cfg(test)]
mod tests;
