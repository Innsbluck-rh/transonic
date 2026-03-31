mod albums;
mod auth;
mod browse;
mod cover_art;
mod session;

#[allow(unused_imports)]
pub use albums::{AlbumListContext, AlbumListItem, AlbumListRequest, AlbumListResponse};
#[allow(unused_imports)]
pub use auth::{AuthInput, AuthKind};
#[allow(unused_imports)]
pub use browse::{
    ArtistIndexItem, ArtistIndexesRequest, ArtistIndexesResponse, FolderStructureAlbumItem,
    FolderStructureAlbumsRequest, FolderStructureAlbumsResponse, FolderStructureRootNode,
    FolderStructureRootsRequest, FolderStructureRootsResponse, FolderStructureSource,
    MusicDirectoryChild, MusicDirectoryRequest, MusicDirectoryResponse, MusicFolderSummary,
    MusicFoldersResponse,
};
#[allow(unused_imports)]
pub use cover_art::{CoverArtRequest, CoverArtResponse};
#[allow(unused_imports)]
pub use session::{
    ActiveSession, AppBootstrap, CapabilityMatrix, ConnectServerProfileRequest,
    ConnectServerProfileResult, LastConnectionState, OpenSubsonicExtension, ProfileIdRequest,
    RestoreStatus, SavedProfileSummary,
};
