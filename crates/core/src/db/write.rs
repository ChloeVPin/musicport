//! High-level catalog mutations: `add_tracks` and `remove_tracks`.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use rusqlite::types::Value;
use rusqlite::params_from_iter;

use super::clone::{kbps, now_mac_epoch};
use super::schema::{Library, Track, TrackRow};

impl Library {
    /// add tracks, returns new pids - clones template rows, caller uploads files
    pub fn add_tracks(&self, tracks: &[Track], folder_path: &str) -> Result<Vec<i64>> {
        self.require_star()?;
        let template_pid = self.template_pid()?.ok_or_else(|| {
            anyhow!(
                "no local song found to use as a template - add at least one song to \
                 the phone via Finder/iTunes first, then retry"
            )
        })?;
        let template_entities = self.template_entities(template_pid)?;
        let base_location_id = self.base_location_id(folder_path)?;
        let container_pid = self.primary_container_pid()?.ok_or_else(|| {
            anyhow!("could not determine the Library container_pid (db_info missing?)")
        })?;

        let mut next_pid = self.max_id("item", "item_pid")? + 1;
        let mut added = Vec::new();
        for track in tracks {
            let new_pid = next_pid;
            next_pid += 1;

            let artist_pid = self.entity_pid(
                "item_artist",
                track.artist.as_deref().unwrap_or("Unknown Artist"),
                &template_entities,
                new_pid,
                &HashMap::new(),
            )?;
            let album_artist_name = track
                .album_artist
                .as_deref()
                .or(track.artist.as_deref())
                .unwrap_or("Unknown Artist");
            let album_artist_pid = self.entity_pid(
                "album_artist",
                album_artist_name,
                &template_entities,
                new_pid,
                &HashMap::new(),
            )?;
            let mut album_extra = HashMap::new();
            album_extra.insert("album_artist_pid".to_string(), Value::Integer(album_artist_pid));
            if let Some(year) = track.year {
                album_extra.insert("album_year".to_string(), Value::Integer(year));
            }
            let album_pid = self.entity_pid(
                "album",
                track.album.as_deref().unwrap_or("Unknown Album"),
                &template_entities,
                new_pid,
                &album_extra,
            )?;
            let genre_id = match &track.genre {
                Some(g) => {
                    self.entity_pid("genre", g, &template_entities, new_pid, &HashMap::new())?
                }
                None => 0,
            };

            let now_mac = now_mac_epoch();
            let mut overrides: HashMap<String, HashMap<String, Value>> = HashMap::new();

            let mut item_ov = HashMap::new();
            item_ov.insert("item_pid".into(), Value::Integer(new_pid));
            item_ov.insert("item_artist_pid".into(), Value::Integer(artist_pid));
            item_ov.insert("album_pid".into(), Value::Integer(album_pid));
            item_ov.insert("album_artist_pid".into(), Value::Integer(album_artist_pid));
            item_ov.insert("genre_id".into(), Value::Integer(genre_id));
            item_ov.insert("base_location_id".into(), Value::Integer(base_location_id));
            // The iOS schema declares these NOT NULL; the reference CLI writes 0 when unset.
            item_ov.insert(
                "track_number".into(),
                Value::Integer(track.track_number.unwrap_or(0)),
            );
            item_ov.insert("disc_number".into(), Value::Integer(track.disc_number.unwrap_or(0)));
            item_ov.insert("keep_local".into(), Value::Integer(1));
            item_ov.insert("in_my_library".into(), Value::Integer(1));
            item_ov.insert("date_added".into(), Value::Integer(now_mac as i64));
            item_ov.insert("date_downloaded".into(), Value::Real(now_mac));
            overrides.insert("item".into(), item_ov);

            // These columns are NOT NULL on iOS; the reference CLI writes 0/'' when unset.
            let mut extra_ov = HashMap::new();
            extra_ov.insert("title".into(), Value::Text(track.title.clone().unwrap_or_default()));
            extra_ov.insert(
                "sort_title".into(),
                Value::Text(track.title.clone().unwrap_or_default()),
            );
            extra_ov.insert("location".into(), Value::Text(track.location.clone()));
            extra_ov.insert("file_size".into(), Value::Integer(track.file_size));
            extra_ov.insert(
                "total_time_ms".into(),
                Value::Real(track.duration_s.map(|s| s * 1000.0).unwrap_or(0.0)),
            );
            extra_ov.insert("year".into(), Value::Integer(track.year.unwrap_or(0)));
            extra_ov.insert("date_modified".into(), Value::Integer(now_mac as i64));
            extra_ov.insert("integrity".into(), Value::Null);
            overrides.insert("item_extra".into(), extra_ov);

            let mut pb_ov = HashMap::new();
            pb_ov.insert("bit_rate".into(), Value::Integer(kbps(track.bitrate.unwrap_or(0))));
            pb_ov.insert("sample_rate".into(), Value::Real(track.sample_rate.unwrap_or(0.0)));
            overrides.insert("item_playback".into(), pb_ov);

            self.clone_item_rowsets(template_pid, new_pid, &overrides)?;
            self.add_container_item(new_pid, container_pid)?;
            added.push(new_pid);
        }
        Ok(added)
    }

    /// delete items + child rows, returns removed rows so caller can delete files too
    pub fn remove_tracks(&self, pids: &[i64]) -> Result<Vec<TrackRow>> {
        self.require_star()?;
        let removed = self.tracks_by_pid(pids)?;
        if pids.is_empty() {
            return Ok(removed);
        }
        let marks = vec!["?"; pids.len()].join(",");
        let mut tables = vec!["item".to_string()];
        tables.extend(super::clone::item_child_tables().iter().map(|s| s.to_string()));
        tables.push("container_item".to_string());
        for table in tables {
            if !self.table_exists(&table)? {
                continue;
            }
            let cols = self.table_columns(&table)?;
            if cols.iter().any(|c| c == "item_pid") {
                self.conn.execute(
                    &format!("DELETE FROM \"{table}\" WHERE item_pid IN ({marks})"),
                    params_from_iter(pids.iter()),
                )?;
            }
        }
        Ok(removed)
    }
}