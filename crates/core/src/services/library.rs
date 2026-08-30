//! library ops: list / add / remove / export tracks

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::db::{self, Library, Track, TrackRow, MUSIC_ROOT};

use super::phone::{backups_dir, stamp, Phone};

impl Phone {
    /// all tracks, optional filter
    pub fn list_tracks(&self, query: Option<&str>) -> Result<Vec<TrackRow>> {
        self.with_library(|lib| lib.all_tracks(query))
    }

    /// Add local audio files to the device library.
    pub fn add_files(&self, files: &[PathBuf], force: bool) -> Result<super::reports::AddReport> {
        let mut tracks: Vec<(Track, String, PathBuf)> = Vec::new();
        let mut messages = Vec::new();
        for f in files {
            match db::tags::read_track(f) {
                Ok(t) => {
                    let ext = f
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_else(|| "mp3".to_string());
                    tracks.push((t, ext, f.clone()));
                }
                Err(e) => messages.push(format!("skipping {}: {e}", f.display())),
            }
        }
        if tracks.is_empty() {
            bail!("no readable audio files given");
        }

        let afc = self.device.open_afc()?;
        let db_path = Self::find_db(&afc)?
            .ok_or_else(|| anyhow!("could not locate the media library DB on the device"))?;

        let backup = self.backup_library(&afc, &db_path)?;
        messages.push(format!("safety snapshot -> {}", backup.display()));

        let work = tempfile::Builder::new().prefix("musicport-").tempdir()?;
        self.download_library(&afc, &db_path, work.path())?;
        let local_db = work
            .path()
            .join(Path::new(&db_path).file_name().unwrap_or_default());
        let lib = Library::open(&local_db)?;

        let mut skipped = 0;
        if !force {
            let existing = lib.all_tracks(None)?;
            let before = tracks.len();
            tracks.retain(|(t, _, _)| {
                !existing.iter().any(|r| {
                    r.title.as_deref().unwrap_or("")
                        .eq_ignore_ascii_case(t.title.as_deref().unwrap_or(""))
                        && r.artist.as_deref().unwrap_or("")
                            .eq_ignore_ascii_case(t.artist.as_deref().unwrap_or(""))
                })
            });
            skipped = before - tracks.len();
            if skipped > 0 {
                messages.push(format!("skipped {skipped} track(s) already in the library"));
            }
        }
        if tracks.is_empty() {
            return Ok(super::reports::AddReport {
                added: 0,
                skipped,
                pids: Vec::new(),
                folder: String::new(),
                messages,
            });
        }

        let folder_name = db::paths::next_folder_name(&lib.folder_names()?)?;
        let folder_path = format!("{MUSIC_ROOT}/{folder_name}");
        let mut existing_files: HashSet<String> = if afc.exists(&folder_path)? {
            afc.listdir(&folder_path)?.into_iter().collect()
        } else {
            afc.make_dir(&folder_path)?;
            HashSet::new()
        };
        for (t, ext, _) in &mut tracks {
            let loc = db::paths::generate_filename(ext, &existing_files)?;
            t.location = loc.clone();
            existing_files.insert(loc);
        }

        let track_slice: Vec<Track> = tracks.iter().map(|(t, _, _)| t.clone()).collect();
        let pids = lib.add_tracks(&track_slice, &folder_path)?;
        drop(lib);

        for (t, _, source) in &tracks {
            let remote = format!("{folder_path}/{}", t.location);
            afc.write_bytes(&remote, &fs::read(source)?)?;
            messages.push(format!("uploaded {remote} ({})", source.display()));
        }

        self.upload_library(&afc, &db_path, &local_db)?;
        let _ = work.close();

        Ok(super::reports::AddReport {
            added: pids.len(),
            skipped,
            pids,
            folder: folder_name,
            messages,
        })
    }

    /// Delete tracks from the device library by pid.
    pub fn remove_tracks(&self, pids: &[i64]) -> Result<super::reports::RemoveReport> {
        if pids.is_empty() {
            bail!("no tracks selected for removal");
        }
        let mut messages = Vec::new();

        let afc = self.device.open_afc()?;
        let db_path = Self::find_db(&afc)?
            .ok_or_else(|| anyhow!("could not locate the media library DB on the device"))?;

        let backup = self.backup_library(&afc, &db_path)?;
        messages.push(format!("safety snapshot -> {}", backup.display()));

        let work = tempfile::Builder::new().prefix("musicport-").tempdir()?;
        self.download_library(&afc, &db_path, work.path())?;
        let local_db = work
            .path()
            .join(Path::new(&db_path).file_name().unwrap_or_default());
        let lib = Library::open(&local_db)?;
        let removed = lib.remove_tracks(pids)?;
        drop(lib);

        for r in &removed {
            if r.location.is_empty() {
                continue;
            }
            let remote = r.remote_path();
            match afc.remove_path(&remote) {
                Ok(()) => messages.push(format!("removed {remote}")),
                Err(e) => messages.push(format!("could not remove file {remote}: {e}")),
            }
        }

        self.upload_library(&afc, &db_path, &local_db)?;
        let _ = work.close();

        Ok(super::reports::RemoveReport {
            removed: removed.len(),
            messages,
        })
    }

    /// Export matching tracks to a local directory with readable filenames.
    pub fn export_tracks(
        &self,
        query: Option<&str>,
        out_dir: &Path,
    ) -> Result<super::reports::ExportReport> {
        fs::create_dir_all(out_dir)?;
        let mut messages = Vec::new();
        let rows = self.with_library(|lib| lib.all_tracks(query))?;

        let afc = self.device.open_afc()?;
        let mut used: HashSet<String> = HashSet::new();
        let mut n = 0usize;
        for r in &rows {
            if r.location.is_empty() {
                continue;
            }
            let remote = r.remote_path();
            let data = match afc.read_bytes(&remote) {
                Ok(d) => d,
                Err(e) => {
                    messages.push(format!(
                        "could not read {remote}: {e} (cloud-only or missing?)"
                    ));
                    continue;
                }
            };
            let ext = Path::new(&r.location)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_else(|| ".mp3".to_string());
            let name = super::naming::unique_name(
                &super::naming::safe_name(r.artist.as_deref(), r.title.as_deref(), &ext),
                &mut used,
            );
            fs::write(out_dir.join(&name), data)?;
            n += 1;
        }
        Ok(super::reports::ExportReport {
            exported: n,
            out_dir: out_dir.display().to_string(),
            messages,
        })
    }

    /// Snapshot the on-device library DB before a write operation.
    fn backup_library(&self, afc: &crate::device::Afc, db_path: &str) -> Result<PathBuf> {
        let dest = backups_dir().join(stamp());
        self.download_library(afc, db_path, &dest)?;
        Ok(dest)
    }
}

/// walk tree and collect Library sqlitedbs (fallback when path is weird)
pub(crate) fn walk_files(
    afc: &crate::device::Afc,
    root: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    match afc.listdir(root) {
        Ok(entries) => {
            for e in entries {
                if e == "." || e == ".." {
                    continue;
                }
                let p = format!("{root}/{e}");
                walk_files(afc, &p, out)?;
            }
            Ok(())
        }
        Err(_) => {
            // Not a directory: it's a file. Keep only library DBs.
            if root.ends_with(".sqlitedb") && root.contains("Library") {
                out.push(root.to_string());
            }
            Ok(())
        }
    }
}