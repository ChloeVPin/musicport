//! Phone handle - connected iphone + helpers for db stuff

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::device::{list_devices, Device, DeviceInfo, DeviceListing};

/// A connected iPhone.
pub struct Phone {
    pub(super) device: Device,
}

impl Phone {
    /// connect to udid or first usb device
    pub fn connect(udid: Option<&str>) -> Result<Self> {
        Ok(Self {
            device: Device::new(udid)?,
        })
    }

    pub fn info(&self) -> Result<DeviceInfo> {
        self.device.info()
    }

    /// Discover devices visible over USB / Wi-Fi (no connection made).
    pub fn discover() -> Result<Vec<DeviceListing>> {
        list_devices()
    }

    // ---- catalog plumbing (used by the operations in `library`) ----

    /// Locate `MediaLibrary.sqlitedb` on the device (path relative to AFC root).
    pub(crate) fn find_db(afc: &crate::device::Afc) -> Result<Option<String>> {
        for cand in [
            "iTunes_Control/iTunes/MediaLibrary.sqlitedb",
            "iTunes_Control/MediaLibrary.sqlitedb",
        ] {
            if afc.exists(cand)? {
                return Ok(Some(cand.to_string()));
            }
        }
        // Fallback: search iTunes_Control for any *Library*.sqlitedb.
        let mut files = Vec::new();
        let _ = super::library::walk_files(afc, "iTunes_Control", &mut files);
        Ok(files.into_iter().next())
    }

    /// download db + wal/shm to dest_dir
    pub(crate) fn download_library(
        &self,
        afc: &crate::device::Afc,
        db_path: &str,
        dest_dir: &Path,
    ) -> Result<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(dest_dir)?;
        let base = Path::new(db_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut saved = Vec::new();
        for suffix in ["", "-wal", "-shm"] {
            let remote = format!("{db_path}{suffix}");
            if afc.exists(&remote)? {
                let local = dest_dir.join(format!("{base}{suffix}"));
                std::fs::write(&local, afc.read_bytes(&remote)?)?;
                saved.push(local);
            }
        }
        Ok(saved)
    }

    /// push db back and delete leftover wal/shm/journal
    pub(crate) fn upload_library(
        &self,
        afc: &crate::device::Afc,
        db_path: &str,
        local_db: &Path,
    ) -> Result<()> {
        afc.write_bytes(db_path, &std::fs::read(local_db)?)?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let remote = format!("{db_path}{suffix}");
            if afc.exists(&remote)? {
                let _ = afc.remove_path(&remote); // best effort
            }
        }
        Ok(())
    }

    /// Download the catalog, run `f` against a local copy, and clean up.
    pub(crate) fn with_library<T>(
        &self,
        f: impl FnOnce(&crate::db::Library) -> Result<T>,
    ) -> Result<T> {
        let afc = self.device.open_afc()?;
        let db_path = Self::find_db(&afc)?
            .ok_or_else(|| anyhow!("could not locate the media library DB on the device"))?;
        let work = tempfile::Builder::new().prefix("musicport-").tempdir()?;
        self.download_library(&afc, &db_path, work.path())?;
        let local_db = work
            .path()
            .join(Path::new(&db_path).file_name().unwrap_or_default());
        let lib = crate::db::Library::open(&local_db)?;
        let out = f(&lib)?;
        drop(lib);
        let _ = work.close();
        Ok(out)
    }
}

/// where backups go: ~/.musicport/backups
pub(crate) fn backups_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    Path::new(&home).join(".musicport").join("backups")
}

/// Ten-digit Unix timestamp used to name a snapshot directory.
pub(crate) fn stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:010}", now.as_secs())
}