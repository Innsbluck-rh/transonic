mod albums;
mod browse;
mod common;
mod cover_art;
mod index;
mod json;
mod session;

pub use albums::get_album_list;
pub use browse::artist::get_music_directory;
pub use browse::folder_structure::get_folder_structure_albums;
pub use cover_art::get_cover_art;
pub use index::artist::get_artist_indexes;
pub use index::folder_structure::{get_folder_structure_roots, get_music_folders};
pub use session::{bootstrap_app_state, connect_server_profile, delete_server_profile};

#[doc(hidden)]
pub use albums::{__cmd__get_album_list, __specta__fn__get_album_list};
#[doc(hidden)]
pub use browse::artist::{__cmd__get_music_directory, __specta__fn__get_music_directory};
#[doc(hidden)]
pub use browse::folder_structure::{
    __cmd__get_folder_structure_albums, __specta__fn__get_folder_structure_albums,
};
#[doc(hidden)]
pub use cover_art::{__cmd__get_cover_art, __specta__fn__get_cover_art};
#[doc(hidden)]
pub use index::artist::{__cmd__get_artist_indexes, __specta__fn__get_artist_indexes};
#[doc(hidden)]
pub use index::folder_structure::{
    __cmd__get_folder_structure_roots, __cmd__get_music_folders,
    __specta__fn__get_folder_structure_roots, __specta__fn__get_music_folders,
};
#[doc(hidden)]
pub use session::{
    __cmd__bootstrap_app_state, __cmd__connect_server_profile, __cmd__delete_server_profile,
    __specta__fn__bootstrap_app_state, __specta__fn__connect_server_profile,
    __specta__fn__delete_server_profile,
};
