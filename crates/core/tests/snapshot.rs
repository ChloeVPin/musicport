//! Integration test against a copy of a REAL MediaLibrary.sqlitedb snapshot
//! (captured via `musicctl inspect` from an iOS 27 device).
//!
//! Gated on the `MUSICCTL_SNAPSHOT` env var pointing at the snapshot file;
//! skipped when unset so `cargo test` works without a device.

use std::env;

use musicport_core::db::{Library, Track};
use rusqlite::Connection;
use tempfile::TempDir;

fn snapshot_path() -> Option<std::path::PathBuf> {
    env::var("MUSICCTL_SNAPSHOT").ok().map(Into::into)
}

fn copy_with_sidecars(src: &std::path::Path, dest_dir: &std::path::Path) -> std::path::PathBuf {
    let work = dest_dir.join("MediaLibrary.sqlitedb");
    std::fs::copy(src, &work).unwrap();
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = src.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = std::path::PathBuf::from(sidecar);
        if sidecar.exists() {
            let dest = dest_dir.join(sidecar.file_name().unwrap());
            std::fs::copy(sidecar, dest).unwrap();
        }
    }
    work
}

#[test]
fn real_snapshot_add_and_remove() {
    let Some(src) = snapshot_path() else {
        eprintln!("skipping: set MUSICCTL_SNAPSHOT=/path/to/MediaLibrary.sqlitedb to run");
        return;
    };
    let tmp = TempDir::new().unwrap();
    let work = copy_with_sidecars(&src, tmp.path());

    let lib = Library::open(&work).unwrap();
    assert!(lib.is_star(), "snapshot must use the star schema");
    let before = lib.all_tracks(None).unwrap().len();
    assert!(before > 0, "snapshot should contain local tracks");

    // Simulate adding one track (as the CLI would).
    let track = Track {
        title: Some("Snapshot Test Song".to_string()),
        artist: Some("musicport-snapshot-artist".to_string()),
        album: Some("Snapshot Test Album".to_string()),
        genre: Some("Test".to_string()),
        track_number: Some(7),
        duration_s: Some(177.7),
        bitrate: Some(320_000),
        sample_rate: Some(44100.0),
        location: "TEST.mp3".to_string(),
        file_size: 1234,
        ..Default::default()
    };
    let added = lib.add_tracks(&[track], "iTunes_Control/Music/F88").unwrap();
    assert_eq!(added.len(), 1);
    let pid = added[0];

    let rows = lib.all_tracks(None).unwrap();
    assert_eq!(rows.len(), before + 1, "one track should be added");
    let new = rows.iter().find(|r| r.pid == pid).expect("new pid present");
    assert_eq!(new.title.as_deref(), Some("Snapshot Test Song"));
    assert_eq!(new.base_path, "iTunes_Control/Music/F88");
    assert_eq!(new.location, "TEST.mp3");
    assert_eq!(new.bit_rate, Some(320));

    // item_state must exist via the DB's own triggers, and the new item must
    // be a member of the Library container.
    let conn = Connection::open(&work).unwrap();
    let state_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM item_state WHERE item_pid = ?1", [pid], |r| r.get(0))
        .unwrap();
    assert_eq!(state_count, 1, "item_state row created by trigger");
    let container_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM container_item WHERE item_pid = ?1",
            [pid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(container_count, 1, "item linked into the Library container");

    // Remove it again - the round trip must be clean.
    let removed = lib.remove_tracks(&[pid]).unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].remote_path(), "iTunes_Control/Music/F88/TEST.mp3");
    assert_eq!(lib.all_tracks(None).unwrap().len(), before);
}
