//! read catalog as TrackRows

use anyhow::Result;
use rusqlite::{params, params_from_iter, Row};

use super::schema::{Library, TrackRow};

/// big join across star schema - what the ui shows
const TRACK_SELECT: &str = "
    SELECT i.item_pid AS pid, ie.title AS title, ia.item_artist AS artist,
           al.album AS album, ie.year AS year, ie.total_time_ms AS duration_ms,
           ip.bit_rate AS bit_rate, ip.sample_rate AS sample_rate,
           bl.path AS base_path, ie.location AS location,
           ie.file_size AS file_size, i.track_number AS track_number,
           i.disc_number AS disc_number
    FROM item i
    JOIN item_extra ie ON ie.item_pid = i.item_pid
    LEFT JOIN item_artist ia ON ia.item_artist_pid = i.item_artist_pid
    LEFT JOIN album al ON al.album_pid = i.album_pid
    LEFT JOIN item_playback ip ON ip.item_pid = i.item_pid
    LEFT JOIN base_location bl ON bl.base_location_id = i.base_location_id
    WHERE bl.path LIKE 'iTunes_Control%'
      AND ie.location IS NOT NULL AND ie.location != ''
";

fn track_from_row(row: &Row) -> rusqlite::Result<TrackRow> {
    Ok(TrackRow {
        pid: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        album: row.get(3)?,
        year: row.get(4)?,
        duration_ms: row.get(5)?,
        bit_rate: row.get(6)?,
        sample_rate: row.get(7)?,
        base_path: row.get(8)?,
        location: row.get(9)?,
        file_size: row.get(10)?,
        track_number: row.get(11)?,
        disc_number: row.get(12)?,
    })
}

impl Library {
    /// all local tracks, optional substring filter
    pub fn all_tracks(&self, query: Option<&str>) -> Result<Vec<TrackRow>> {
        self.require_star()?;
        let mut sql = TRACK_SELECT.to_string();
        if let Some(q) = query {
            sql.push_str(" AND (ie.title LIKE ?1 OR ia.item_artist LIKE ?1 OR al.album LIKE ?1)");
            let like = format!("%{q}%");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params![like], track_from_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        } else {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map([], track_from_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        }
    }

    pub fn tracks_by_pid(&self, pids: &[i64]) -> Result<Vec<TrackRow>> {
        if pids.is_empty() {
            return Ok(Vec::new());
        }
        self.require_star()?;
        let marks = vec!["?"; pids.len()].join(",");
        let sql = format!("{TRACK_SELECT} AND i.item_pid IN ({marks})");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(pids.iter()), track_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}