//! Unit tests for the star-schema media library logic (no device needed).
//! Mirrors musicctl/tests/test_dblib.py.

use std::path::Path;

use musicport_core::db::{Library, Track};
use rusqlite::Connection;
use tempfile::TempDir;

const PRIMARY_CONTAINER: i64 = 6330457520395302423;
const TEMPLATE_PID: i64 = 183;
const TEMPLATE_ARTIST_PID: i64 = 33554462;
const TEMPLATE_ALBUM_PID: i64 = 67108901;
const TEMPLATE_ALBUM_ARTIST_PID: i64 = 117440526;
const TEMPLATE_GENRE_ID: i64 = 50331649;
const TEMPLATE_BASE_LOCATION: i64 = 3888;

fn make_db(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE item (item_pid INTEGER PRIMARY KEY, media_type INTEGER, title_order INTEGER,
            item_artist_pid INTEGER, album_pid INTEGER, album_artist_pid INTEGER, genre_id INTEGER,
            base_location_id INTEGER, track_number INTEGER, disc_number INTEGER,
            date_added INTEGER, date_downloaded REAL, keep_local INTEGER, in_my_library INTEGER);
        CREATE TABLE item_extra (item_pid INTEGER PRIMARY KEY, title TEXT, sort_title TEXT,
            total_time_ms REAL, year INTEGER, location TEXT, file_size INTEGER,
            date_modified INTEGER, media_kind INTEGER, integrity BLOB);
        CREATE TABLE item_playback (item_pid INTEGER PRIMARY KEY, audio_format INTEGER,
            bit_rate INTEGER, sample_rate REAL, duration REAL);
        CREATE TABLE item_state (item_pid INTEGER PRIMARY KEY, persistent_id INTEGER,
            title TEXT, location TEXT, is_placeholder INTEGER, is_protected INTEGER);
        CREATE TABLE item_search (item_pid INTEGER PRIMARY KEY, search_title INTEGER, search_artist INTEGER);
        CREATE TABLE item_stats (item_pid INTEGER PRIMARY KEY, play_count_user INTEGER, date_accessed REAL);
        CREATE TABLE item_artist (item_artist_pid INTEGER PRIMARY KEY, item_artist TEXT,
            sort_item_artist TEXT, grouping_key BLOB, cloud_status INTEGER, representative_item_pid INTEGER);
        CREATE TABLE album (album_pid INTEGER PRIMARY KEY, album TEXT, sort_album TEXT,
            album_artist_pid INTEGER, grouping_key BLOB, album_year INTEGER, cloud_status INTEGER,
            representative_item_pid INTEGER);
        CREATE TABLE album_artist (album_artist_pid INTEGER PRIMARY KEY, album_artist TEXT,
            sort_album_artist TEXT, grouping_key BLOB, representative_item_pid INTEGER,
            sort_order INTEGER, name_order INTEGER);
        CREATE TABLE genre (genre_id INTEGER PRIMARY KEY, genre TEXT, grouping_key BLOB,
            representative_item_pid INTEGER);
        CREATE TABLE base_location (base_location_id INTEGER PRIMARY KEY, path TEXT);
        CREATE TABLE container_item (container_item_pid INTEGER PRIMARY KEY, container_pid INTEGER,
            item_pid INTEGER, position INTEGER, uuid TEXT, position_uuid TEXT, occurrence_id TEXT);
        CREATE TABLE db_info (db_pid INTEGER, primary_container_pid INTEGER);
        INSERT INTO db_info VALUES (-1, 6330457520395302423);
        INSERT INTO base_location VALUES (3888, 'iTunes_Control/Music/F48');
        INSERT INTO item_artist VALUES (33554462, 'Template Artist', 'Template Artist', X'00', 0, 183);
        INSERT INTO album VALUES (67108901, 'Template Album', 'Template Album', 117440526, X'00', 2016, 0, 183);
        INSERT INTO album_artist VALUES (117440526, 'Template Artist', 'Template Artist', X'00', 183, 0, 0);
        INSERT INTO genre VALUES (50331649, 'Hip-Hop', X'00', 183);
        INSERT INTO item VALUES (183, 8, 4170413244416, 33554462, 67108901, 117440526, 50331649, 3888, 1, 1, 800397205, 800398080.86, 1, 1);
        INSERT INTO item_extra VALUES (183, 'Template Song', 'Template Song', 251402.0, 2016, 'OSLF.mp3', 10264659, 800334393, 1, X'00');
        INSERT INTO item_playback VALUES (183, 301, 320, 44100.0, 0.0);
        INSERT INTO item_state VALUES (183, 183, 'Template Song', 'OSLF.mp3', 0, 0);
        INSERT INTO item_search VALUES (183, 4170413244416, 4866197946368);
        INSERT INTO item_stats VALUES (183, 0, 800398080.86);
        INSERT INTO container_item VALUES (150997135, 6330457520395302423, 183, 0, '', '', '183_0');

        -- Real sync triggers: inserting into item / item_extra maintains item_state.
        CREATE TRIGGER item_state_insert_sync AFTER INSERT ON item FOR EACH ROW BEGIN
            INSERT INTO item_state (item_pid, persistent_id, title, location, is_placeholder, is_protected)
            VALUES (NEW.item_pid, NEW.item_pid, '', '', 0, 0);
        END;
        CREATE TRIGGER item_state_extra_insert_sync AFTER INSERT ON item_extra FOR EACH ROW BEGIN
            INSERT INTO item_state (item_pid, persistent_id, title, location)
            VALUES (NEW.item_pid, NEW.item_pid, COALESCE(NEW.title, ''), COALESCE(NEW.location, ''))
            ON CONFLICT(item_pid) DO UPDATE SET title = excluded.title, location = excluded.location;
        END;
        CREATE TRIGGER item_state_delete_sync AFTER DELETE ON item FOR EACH ROW BEGIN
            DELETE FROM item_state WHERE item_pid = OLD.item_pid;
        END;
        "#,
    )
    .unwrap();
    conn.close();
}

fn make_track(name: &str) -> Track {
    Track {
        title: Some(name.to_string()),
        artist: Some("New Artist".to_string()),
        album: Some("New Album".to_string()),
        genre: Some("Hip-Hop".to_string()),
        track_number: Some(2),
        duration_s: Some(180.5),
        bitrate: Some(192_000),
        sample_rate: Some(44100.0),
        location: "ABCD.mp3".to_string(),
        file_size: 1000,
        ..Default::default()
    }
}

struct TestLib {
    _tmp: TempDir,
    lib: Library,
}

impl TestLib {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("MediaLibrary.sqlitedb");
        make_db(&db_path);
        let lib = Library::open(&db_path).unwrap();
        Self { _tmp: tmp, lib }
    }
}

#[test]
fn add_track_deep_clone() {
    let t = TestLib::new();
    let track = make_track("New Song");
    let added = t.lib.add_tracks(&[track], "iTunes_Control/Music/F88").unwrap();
    assert_eq!(added, vec![TEMPLATE_PID + 1]);

    let rows = t.lib.all_tracks(None).unwrap();
    assert_eq!(rows.len(), 2);
    let new = rows.iter().find(|r| r.pid == TEMPLATE_PID + 1).unwrap();
    assert_eq!(new.title.as_deref(), Some("New Song"));
    assert_eq!(new.artist.as_deref(), Some("New Artist"));
    assert_eq!(new.album.as_deref(), Some("New Album"));
    assert_eq!(new.base_path, "iTunes_Control/Music/F88");
    assert_eq!(new.location, "ABCD.mp3");
    assert_eq!(new.duration_ms, Some(180500.0));
    assert_eq!(new.bit_rate, Some(192)); // kbps
    assert_eq!(new.remote_path(), "iTunes_Control/Music/F88/ABCD.mp3");

    // item_state is maintained by the DB's own sync triggers.
    let conn = Connection::open(t.lib.db_path()).unwrap();
    let state: (i64, String, String) = conn
        .query_row(
            "SELECT persistent_id, title, location FROM item_state WHERE item_pid = ?1",
            [TEMPLATE_PID + 1],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, (TEMPLATE_PID + 1, "New Song".to_string(), "ABCD.mp3".to_string()));
}

#[test]
fn add_reuses_existing_entities_by_name() {
    let t = TestLib::new();
    let mut track = make_track("Song A");
    track.artist = Some("Template Artist".to_string());
    track.album = Some("Template Album".to_string());
    t.lib.add_tracks(&[track], "iTunes_Control/Music/F88").unwrap();
    // No new entity rows should be created for the existing names.
    let conn = Connection::open(t.lib.db_path()).unwrap();
    let max_artist: i64 = conn
        .query_row("SELECT MAX(item_artist_pid) FROM item_artist", [], |r| r.get(0))
        .unwrap();
    let max_album: i64 = conn.query_row("SELECT MAX(album_pid) FROM album", [], |r| r.get(0)).unwrap();
    assert_eq!(max_artist, TEMPLATE_ARTIST_PID);
    assert_eq!(max_album, TEMPLATE_ALBUM_PID);
}

#[test]
fn add_creates_new_entities_with_bit_encoded_pids() {
    let t = TestLib::new();
    let track = make_track("Song B");
    t.lib.add_tracks(&[track], "iTunes_Control/Music/F88").unwrap();
    let conn = Connection::open(t.lib.db_path()).unwrap();
    let max_artist: i64 = conn
        .query_row("SELECT MAX(item_artist_pid) FROM item_artist", [], |r| r.get(0))
        .unwrap();
    let max_album: i64 = conn.query_row("SELECT MAX(album_pid) FROM album", [], |r| r.get(0)).unwrap();
    let max_aa: i64 = conn
        .query_row("SELECT MAX(album_artist_pid) FROM album_artist", [], |r| r.get(0))
        .unwrap();
    assert!(max_artist > TEMPLATE_ARTIST_PID);
    assert!(max_album > TEMPLATE_ALBUM_PID);
    assert!(max_aa > TEMPLATE_ALBUM_ARTIST_PID);
}

#[test]
fn add_links_into_library_container() {
    let t = TestLib::new();
    let track = make_track("Song C");
    let added = t.lib.add_tracks(&[track], "iTunes_Control/Music/F88").unwrap();
    let conn = Connection::open(t.lib.db_path()).unwrap();
    let (container_pid, occurrence): (i64, String) = conn
        .query_row(
            "SELECT container_pid, occurrence_id FROM container_item WHERE item_pid = ?1",
            [added[0]],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(container_pid, PRIMARY_CONTAINER);
    assert_eq!(occurrence, format!("{}_0", added[0]));
}

#[test]
fn remove_track_deletes_all_rows_and_returns_remote_paths() {
    let t = TestLib::new();
    let removed = t.lib.remove_tracks(&[TEMPLATE_PID]).unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].remote_path(), "iTunes_Control/Music/F48/OSLF.mp3");
    assert!(t.lib.all_tracks(None).unwrap().is_empty());
    let conn = Connection::open(t.lib.db_path()).unwrap();
    for table in [
        "item",
        "item_extra",
        "item_playback",
        "item_state",
        "item_search",
        "item_stats",
        "container_item",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "{table} not cleaned up");
    }
}

#[test]
fn query_filter() {
    let t = TestLib::new();
    let track = make_track("Unicorn");
    t.lib.add_tracks(&[track], "iTunes_Control/Music/F88").unwrap();
    assert_eq!(t.lib.all_tracks(Some("unicorn")).unwrap().len(), 1);
    assert_eq!(t.lib.all_tracks(Some("new artist")).unwrap().len(), 1);
    assert_eq!(t.lib.all_tracks(Some("nope")).unwrap().len(), 0);
}

#[test]
fn folder_names() {
    let t = TestLib::new();
    assert_eq!(t.lib.folder_names().unwrap(), ["F48"].into_iter().map(str::to_string).collect());
}
