//! cloning logic for writes
//! copy an existing song's rows to new pid and patch a few cols - unknown cols
//! just copy over so ios updates don't break

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use rusqlite::types::Value;
use rusqlite::{params, ToSql};

use super::schema::Library;
use super::{ENTITY_TABLES, MAC_EPOCH_OFFSET};

/// child tables we clone with item - no item_state, triggers create it
const ITEM_CHILD_TABLES: &[&str] = &[
    "item_extra",
    "item_playback",
    "item_search",
    "item_stats",
    "item_store",
    "item_video",
    "item_kvs",
    "lyrics",
    "chapter",
    "booklet",
];

impl Library {
    /// newest local track to use as template
    pub(crate) fn template_pid(&self) -> Result<Option<i64>> {
        let row = self.conn.query_row(
            "SELECT i.item_pid FROM item i \
             JOIN item_extra ie ON ie.item_pid = i.item_pid \
             JOIN base_location bl ON bl.base_location_id = i.base_location_id \
             WHERE bl.path LIKE 'iTunes_Control%' AND ie.location != '' \
               AND ie.title IS NOT NULL AND ie.title != '' \
             ORDER BY i.item_pid DESC LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        );
        match row {
            Ok(pid) => Ok(Some(pid)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// grab template's entity rows so we can clone them
    pub(crate) fn template_entities(
        &self,
        template_pid: i64,
    ) -> Result<HashMap<String, (Vec<String>, Vec<Value>)>> {
        let item = self.select_row("item", "WHERE item_pid = ?1", &[&template_pid])?;
        let mut out = HashMap::new();
        for (table, pid_col, _name_col) in ENTITY_TABLES {
            let mut template = None;
            if let Some((cols, vals)) = &item {
                if let Some(Value::Integer(pid)) = Library::col_value(cols, vals, pid_col) {
                    template =
                        self.select_row(table, &format!("WHERE {pid_col} = ?1"), &[&*pid])?;
                }
            }
            if template.is_none() {
                // Fall back to any existing row of this entity table.
                template = self.select_row(table, "", &[] as &[&dyn ToSql])?;
            }
            if let Some(t) = template {
                out.insert(table.to_string(), t);
            }
        }
        Ok(out)
    }

    /// find entity pid or make a new one from template
    pub(crate) fn entity_pid(
        &self,
        table: &str,
        name: &str,
        template_entities: &HashMap<String, (Vec<String>, Vec<Value>)>,
        representative_pid: i64,
        extra: &HashMap<String, Value>,
    ) -> Result<i64> {
        let (pid_col, name_col) = ENTITY_TABLES
            .iter()
            .find(|(t, _, _)| *t == table)
            .map(|(_, pid, name)| (*pid, *name))
            .ok_or_else(|| anyhow!("unknown entity table {table}"))?;
        let name = if name.trim().is_empty() { "Unknown" } else { name };

        let existing = self.conn.query_row(
            &format!("SELECT {pid_col} FROM {table} WHERE lower({name_col}) = lower(?1) LIMIT 1"),
            [name],
            |r| r.get::<_, i64>(0),
        );
        if let Ok(pid) = existing {
            return Ok(pid);
        }

        let (tpl_cols, tpl_vals) = template_entities
            .get(table)
            .ok_or_else(|| anyhow!("cannot create a new {table} row: no template row available"))?;
        let cols = tpl_cols.clone();
        let mut vals = tpl_vals.clone();
        Library::set_col(
            &cols,
            &mut vals,
            pid_col,
            Value::Integer(self.max_id(table, pid_col)? + 1),
        );
        Library::set_col(&cols, &mut vals, name_col, Value::Text(name.to_string()));
        Library::set_col(
            &cols,
            &mut vals,
            &format!("sort_{name_col}"),
            Value::Text(name.to_string()),
        );
        Library::set_col(
            &cols,
            &mut vals,
            "representative_item_pid",
            Value::Integer(representative_pid),
        );
        for (key, value) in extra {
            Library::set_col(&cols, &mut vals, key, value.clone());
        }
        self.insert_row(table, &cols, &vals)?;
        match Library::col_value(&cols, &vals, pid_col) {
            Some(Value::Integer(pid)) => Ok(*pid),
            _ => bail!("new {table} row has a non-integer {pid_col}"),
        }
    }

    pub(crate) fn base_location_id(&self, folder_path: &str) -> Result<i64> {
        let existing = self.conn.query_row(
            "SELECT base_location_id FROM base_location WHERE path = ?1 LIMIT 1",
            [folder_path],
            |r| r.get::<_, i64>(0),
        );
        if let Ok(id) = existing {
            return Ok(id);
        }
        let new_id = self.max_id("base_location", "base_location_id")? + 1;
        self.conn.execute(
            "INSERT INTO base_location (base_location_id, path) VALUES (?1, ?2)",
            params![new_id, folder_path],
        )?;
        Ok(new_id)
    }

    pub(crate) fn primary_container_pid(&self) -> Result<Option<i64>> {
        let row = self.conn.query_row(
            "SELECT primary_container_pid FROM db_info LIMIT 1",
            [],
            |r| r.get::<_, Option<i64>>(0),
        );
        if let Ok(Some(pid)) = row {
            return Ok(Some(pid));
        }
        let fallback = self.conn.query_row(
            "SELECT container_pid FROM container_item GROUP BY container_pid \
             ORDER BY COUNT(*) DESC LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        );
        match fallback {
            Ok(pid) => Ok(Some(pid)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) fn add_container_item(&self, new_pid: i64, container_pid: i64) -> Result<()> {
        let cip = self.max_id("container_item", "container_item_pid")? + 1;
        let position = self
            .conn
            .query_row(
                "SELECT MAX(position) FROM container_item WHERE container_pid = ?1",
                [container_pid],
                |r| r.get::<_, Option<i64>>(0),
            )?
            .unwrap_or(0)
            + 1;
        self.conn.execute(
            "INSERT INTO container_item (container_item_pid, container_pid, item_pid, \
             position, uuid, position_uuid, occurrence_id) VALUES (?1, ?2, ?3, ?4, '', '', ?5)",
            params![cip, container_pid, new_pid, position, format!("{new_pid}_0")],
        )?;
        Ok(())
    }

    /// clone template rows to new pid, apply overrides per table
    pub(crate) fn clone_item_rowsets(
        &self,
        template_pid: i64,
        new_pid: i64,
        overrides: &HashMap<String, HashMap<String, Value>>,
    ) -> Result<()> {
        for table in std::iter::once("item").chain(ITEM_CHILD_TABLES.iter().copied()) {
            if !self.table_exists(table)? {
                continue;
            }
            let cols = self.table_columns(table)?;
            let rows = self.select_all_values(table, "WHERE item_pid = ?1", &[&template_pid])?;
            for mut vals in rows {
                Library::set_col(&cols, &mut vals, "item_pid", Value::Integer(new_pid));
                if let Some(ov) = overrides.get(table) {
                    for (key, value) in ov {
                        Library::set_col(&cols, &mut vals, key, value.clone());
                    }
                }
                if table == "booklet" {
                    Library::set_col(
                        &cols,
                        &mut vals,
                        "booklet_pid",
                        Value::Integer(self.max_id("booklet", "booklet_pid")? + 1),
                    );
                }
                self.insert_row(table, &cols, &vals)?;
            }
        }
        Ok(())
    }
}

/// Item-keyed satellite tables cloned alongside a new item row.
pub(crate) fn item_child_tables() -> &'static [&'static str] {
    ITEM_CHILD_TABLES
}

/// Seconds since the Mac (Cocoa) epoch for the current time.
pub(crate) fn now_mac_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() - MAC_EPOCH_OFFSET)
        .unwrap_or(0.0)
}

/// Bitrate in bits/sec (as reported by tag libraries) -> DB storage (kbps).
pub(crate) fn kbps(bitrate: i64) -> i64 {
    if bitrate > 10000 {
        bitrate / 1000
    } else {
        bitrate
    }
}