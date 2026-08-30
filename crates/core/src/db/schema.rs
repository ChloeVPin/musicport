//! Library handle + introspection
//! opens local MediaLibrary.sqlitedb, checks star vs flat schema, low-level row helpers

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, ToSql};

/// Audio metadata for a track to add to the library.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Track {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub duration_s: Option<f64>,
    /// Bitrate in bits/sec (as reported by audio tag libraries).
    pub bitrate: Option<i64>,
    pub sample_rate: Option<f64>,
    /// Filename on the device (e.g. "ABCD.mp3").
    pub location: String,
    pub file_size: i64,
}

/// One local track as seen through the catalog joins.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TrackRow {
    pub pid: i64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i64>,
    pub duration_ms: Option<f64>,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<f64>,
    pub base_path: String,
    pub location: String,
    pub file_size: Option<i64>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
}

impl TrackRow {
    /// Full on-device path (relative to the AFC root) for this track.
    pub fn remote_path(&self) -> String {
        format!("{}/{}", self.base_path.trim_end_matches('/'), self.location)
    }
}

/// Read/write access to a local copy of `MediaLibrary.sqlitedb`.
pub struct Library {
    pub(crate) conn: Connection,
    db_path: PathBuf,
}

impl Library {
    /// open a downloaded db file
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self {
            conn,
            db_path: path.to_path_buf(),
        })
    }

    /// Path of the local DB file this handle was opened against.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // ---- schema detection ----

    /// true if this db uses star schema (has item.item_pid)
    pub fn is_star(&self) -> bool {
        self.item_columns()
            .map(|cols| cols.iter().any(|c| c == "item_pid"))
            .unwrap_or(false)
    }

    pub fn schema_name(&self) -> &'static str {
        if self.is_star() {
            "star (iOS 27+)"
        } else {
            "flat (older iOS)"
        }
    }

    /// require star schema or bail
    pub fn require_star(&self) -> Result<()> {
        if !self.is_star() {
            bail!(
                "this library DB uses the older flat schema, which is not supported for \
                 writes yet. Detected schema: {}",
                self.schema_name()
            );
        }
        Ok(())
    }

    // ---- introspection ----

    /// all tables -> columns, sorted
    pub fn tables(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut out = HashMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for name in names {
            out.insert(name.clone(), self.table_columns(&name)?);
        }
        Ok(out)
    }

    pub fn item_columns(&self) -> Result<Vec<String>> {
        self.table_columns("item")
    }

    pub fn item_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM item", [], |r| r.get(0))?)
    }

    /// which Fxx folders exist, eg F00
    pub fn folder_names(&self) -> Result<HashSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT path FROM base_location WHERE path LIKE 'iTunes_Control/Music/%'",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = HashSet::new();
        for row in rows {
            let path = row?;
            if let Some(seg) = path.rsplit('/').next() {
                out.insert(seg.to_string());
            }
        }
        Ok(out)
    }

    // ---- shared row primitives (used by query / clone / write) ----

    pub(crate) fn table_exists(&self, table: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1")?;
        Ok(stmt.exists([table])?)
    }

    pub(crate) fn table_columns(&self, table: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info(\"{table}\")"))?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn max_id(&self, table: &str, column: &str) -> Result<i64> {
        let sql = format!("SELECT MAX({column}) FROM {table}");
        Ok(self
            .conn
            .query_row(&sql, [], |r| r.get::<_, Option<i64>>(0))?
            .unwrap_or(0))
    }

    /// get one row as (cols, vals) or None - generic Value so extra cols don't matter
    pub(crate) fn select_row(
        &self,
        table: &str,
        where_sql: &str,
        params: &[&dyn ToSql],
    ) -> Result<Option<(Vec<String>, Vec<Value>)>> {
        let cols = self.table_columns(table)?;
        let n = cols.len();
        let sql = format!("SELECT * FROM {table} {where_sql}");
        let mut stmt = self.conn.prepare(&sql)?;
        let row = stmt.query_row(params, move |row| {
            (0..n)
                .map(|i| row.get::<_, Value>(i))
                .collect::<rusqlite::Result<Vec<Value>>>()
        });
        match row {
            Ok(vals) => Ok(Some((cols, vals))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) fn select_all_values(
        &self,
        table: &str,
        where_sql: &str,
        params: &[&dyn ToSql],
    ) -> Result<Vec<Vec<Value>>> {
        let cols = self.table_columns(table)?;
        let n = cols.len();
        let sql = format!("SELECT * FROM {table} {where_sql}");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params, move |row| {
            (0..n)
                .map(|i| row.get::<_, Value>(i))
                .collect::<rusqlite::Result<Vec<Value>>>()
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// insert one row, cols order matters
    pub(crate) fn insert_row(
        &self,
        table: &str,
        cols: &[String],
        vals: &[Value],
    ) -> Result<()> {
        let col_list = cols.join(",");
        let marks = vec!["?"; cols.len()].join(",");
        self.conn.execute(
            &format!("INSERT INTO {table} ({col_list}) VALUES ({marks})"),
            params_from_iter(vals.iter()),
        )?;
        Ok(())
    }

    /// Set a column's value by name, no-op if the column doesn't exist.
    pub(crate) fn set_col(cols: &[String], vals: &mut [Value], name: &str, v: Value) {
        if let Some(i) = cols.iter().position(|c| c == name) {
            if i < vals.len() {
                vals[i] = v;
            }
        }
    }

    pub(crate) fn col_value<'a>(
        cols: &[String],
        vals: &'a [Value],
        name: &str,
    ) -> Option<&'a Value> {
        cols.iter().position(|c| c == name).and_then(|i| vals.get(i))
    }
}