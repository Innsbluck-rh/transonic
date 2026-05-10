mod albums;
mod artist_image;
mod backup;
mod browse;
pub(crate) mod common;
mod connect;
mod cover_art;
mod index;
mod json;
pub(crate) mod playback;
mod session;
mod settings;

pub use albums::get_album_list;
pub use artist_image::get_artist_image;
pub use backup::{export_server_backup, import_server_backup};
pub use browse::album::{get_album_info, get_album_songs};
pub use browse::artist::{get_artist, get_artist_info2, get_artists, get_music_directory};
pub use browse::folder_structure::get_folder_structure_albums;
pub use browse::folder_structure_album_songs::get_folder_structure_album_songs;
pub use browse::song::get_song;
pub use connect::{
    connect_get_devices_with_playback, connect_get_runtime_status, connect_pause_device_playback,
    connect_takeover_playback,
};
pub use cover_art::{clear_cover_art_cache, get_cover_art, get_cover_art_cache_status};
pub use index::artist::get_artist_indexes;
pub use index::folder_structure::{get_folder_structure_roots, get_music_folders};
pub use playback::{
    consume_pending_notification_tap, playback_append_to_queue, playback_get_state,
    playback_insert_after_current, playback_next, playback_pause, playback_play,
    playback_play_queue_index, playback_prev, playback_seek, playback_set_position,
    playback_set_queue, playback_stop,
};
pub use session::{bootstrap_app_state, connect_server_profile, delete_server_profile};
pub use settings::{get_default_device_name, settings_update};

#[doc(hidden)]
pub use albums::{__cmd__get_album_list, __specta__fn__get_album_list};
#[doc(hidden)]
pub use artist_image::{__cmd__get_artist_image, __specta__fn__get_artist_image};
#[doc(hidden)]
pub use backup::{
    __cmd__export_server_backup, __cmd__import_server_backup, __specta__fn__export_server_backup,
    __specta__fn__import_server_backup,
};
#[doc(hidden)]
pub use browse::album::{
    __cmd__get_album_info, __cmd__get_album_songs, __specta__fn__get_album_info,
    __specta__fn__get_album_songs,
};
#[doc(hidden)]
pub use browse::artist::{
    __cmd__get_artist, __cmd__get_artist_info2, __cmd__get_artists, __cmd__get_music_directory,
    __specta__fn__get_artist, __specta__fn__get_artist_info2, __specta__fn__get_artists,
    __specta__fn__get_music_directory,
};
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
pub use connect::{
    __cmd__connect_get_devices_with_playback, __cmd__connect_get_runtime_status,
    __cmd__connect_pause_device_playback, __cmd__connect_takeover_playback,
    __specta__fn__connect_get_devices_with_playback, __specta__fn__connect_get_runtime_status,
    __specta__fn__connect_pause_device_playback, __specta__fn__connect_takeover_playback,
};
#[doc(hidden)]
pub use cover_art::{
    __cmd__clear_cover_art_cache, __cmd__get_cover_art, __cmd__get_cover_art_cache_status,
    __specta__fn__clear_cover_art_cache, __specta__fn__get_cover_art,
    __specta__fn__get_cover_art_cache_status,
};
#[doc(hidden)]
pub use index::artist::{__cmd__get_artist_indexes, __specta__fn__get_artist_indexes};
#[doc(hidden)]
pub use index::folder_structure::{
    __cmd__get_folder_structure_roots, __cmd__get_music_folders,
    __specta__fn__get_folder_structure_roots, __specta__fn__get_music_folders,
};
#[doc(hidden)]
pub use playback::{
    __cmd__consume_pending_notification_tap, __cmd__playback_append_to_queue,
    __cmd__playback_get_state, __cmd__playback_insert_after_current, __cmd__playback_next,
    __cmd__playback_pause, __cmd__playback_play, __cmd__playback_play_queue_index,
    __cmd__playback_prev, __cmd__playback_seek, __cmd__playback_set_position,
    __cmd__playback_set_queue, __cmd__playback_stop,
    __specta__fn__consume_pending_notification_tap, __specta__fn__playback_append_to_queue,
    __specta__fn__playback_get_state, __specta__fn__playback_insert_after_current,
    __specta__fn__playback_next, __specta__fn__playback_pause, __specta__fn__playback_play,
    __specta__fn__playback_play_queue_index, __specta__fn__playback_prev,
    __specta__fn__playback_seek, __specta__fn__playback_set_position,
    __specta__fn__playback_set_queue, __specta__fn__playback_stop,
};
#[doc(hidden)]
pub use session::{
    __cmd__bootstrap_app_state, __cmd__connect_server_profile, __cmd__delete_server_profile,
    __specta__fn__bootstrap_app_state, __specta__fn__connect_server_profile,
    __specta__fn__delete_server_profile,
};
#[doc(hidden)]
pub use settings::{
    __cmd__get_default_device_name, __cmd__settings_update, __specta__fn__get_default_device_name,
    __specta__fn__settings_update,
};
