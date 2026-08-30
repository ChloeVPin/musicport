//! read/write MediaLibrary.sqlitedb (local copy)
//! port of musicctl python. writes clone an existing song's rows and patch a few
//! fields - unknown cols just copy over so schema drift doesn't break us.
//! layers: schema -> query -> clone -> write -> paths/tags

pub mod clone;
pub mod paths;
pub mod query;
pub mod schema;
pub mod tags;
pub mod write;

pub use schema::{Library, Track, TrackRow};

pub const MUSIC_ROOT: &str = "iTunes_Control/Music";
/// secs between unix epoch and mac epoch (2001) - ios stores timestamps vs mac epoch
pub const MAC_EPOCH_OFFSET: f64 = 978307200.0;

/// entity tables (artist, album etc) - reused or cloned for new tracks
pub(crate) const ENTITY_TABLES: &[(&str, &str, &str)] = &[
    ("item_artist", "item_artist_pid", "item_artist"),
    ("album", "album_pid", "album"),
    ("album_artist", "album_artist_pid", "album_artist"),
    ("genre", "genre_id", "genre"),
];