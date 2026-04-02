mod albums;
mod browse;
pub(crate) mod common;
mod cover_art;
mod index;
mod json;
mod playback;
mod session;

pub use albums::get_album_list;
pub use browse::album::get_album_songs;
pub use browse::artist::get_music_directory;
pub use browse::folder_structure::get_folder_structure_albums;
pub use browse::folder_structure_album_songs::get_folder_structure_album_songs;
pub use browse::song::get_song;
pub use cover_art::get_cover_art;
pub use index::artist::get_artist_indexes;
pub use index::folder_structure::{get_folder_structure_roots, get_music_folders};
pub use playback::{
    playback_get_state, playback_next, playback_pause, playback_play, playback_play_album,
    playback_play_folder_album, playback_play_queue_index, playback_prev, playback_seek,
    playback_set_queue, playback_stop,
};
pub use session::{bootstrap_app_state, connect_server_profile, delete_server_profile};

#[doc(hidden)]
pub use albums::{__cmd__get_album_list, __specta__fn__get_album_list};
#[doc(hidden)]
pub use browse::album::{__cmd__get_album_songs, __specta__fn__get_album_songs};
#[doc(hidden)]
pub use browse::artist::{__cmd__get_music_directory, __specta__fn__get_music_directory};
#[doc(hidden)]
pub use browse::folder_structure::{
    __cmd__get_folder_structure_albums, __specta__fn__get_folder_structure_albums,
};
#[doc(hidden)]
pub use browse::folder_structure_album_songs::{
    __cmd__get_folder_structure_album_songs, __specta__fn__get_folder_structure_album_songs,
};
#[doc(hidden)]
pub use browse::song::{__cmd__get_song, __specta__fn__get_song};
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
pub use playback::{
    __cmd__playback_get_state, __cmd__playback_next, __cmd__playback_pause, __cmd__playback_play,
    __cmd__playback_play_album, __cmd__playback_play_folder_album,
    __cmd__playback_play_queue_index, __cmd__playback_prev, __cmd__playback_seek,
    __cmd__playback_set_queue, __cmd__playback_stop, __specta__fn__playback_get_state,
    __specta__fn__playback_next, __specta__fn__playback_pause, __specta__fn__playback_play,
    __specta__fn__playback_play_album, __specta__fn__playback_play_folder_album,
    __specta__fn__playback_play_queue_index, __specta__fn__playback_prev,
    __specta__fn__playback_seek, __specta__fn__playback_set_queue, __specta__fn__playback_stop,
};
#[doc(hidden)]
pub use session::{
    __cmd__bootstrap_app_state, __cmd__connect_server_profile, __cmd__delete_server_profile,
    __specta__fn__bootstrap_app_state, __specta__fn__connect_server_profile,
    __specta__fn__delete_server_profile,
};
