//! helpers for ios music folders (Fxx/XXXX.ext) - port of paths.py
//! names are random hex, just need to be unique

use std::collections::HashSet;

use anyhow::{bail, Result};
use rand::Rng;

/// Generate a unique `XXXX.ext` filename not colliding with `existing`.
pub fn generate_filename(ext: &str, existing: &HashSet<String>) -> Result<String> {
    let ext = normalize_ext(ext);
    let mut rng = rand::thread_rng();
    for _ in 0..200 {
        let name: String = (0..4).map(|_| format!("{:X}", rng.gen_range(0..16))).collect();
        let name = format!("{name}{ext}");
        if !existing.contains(&name) {
            return Ok(name);
        }
    }
    bail!("could not generate a unique media filename")
}

/// The next folder bucket name (F00, F01, ..., FFF) after the existing ones.
pub fn next_folder_name(existing: &HashSet<String>) -> Result<String> {
    let mut max: Option<u32> = None;
    for f in existing {
        // Bucket names are F followed by exactly 2 hex digits (F00 … FFF).
        if let Some(hex) = f.strip_prefix('F') {
            if hex.len() == 2 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(n) = u32::from_str_radix(hex, 16) {
                    max = Some(max.map_or(n, |m| m.max(n)));
                }
            }
        }
    }
    let nxt = max.map_or(0, |m| m + 1);
    // old cli checked >0xFFF but that never fires for 2-digit buckets - we bail at 0xFF
    if nxt > 0xFF {
        bail!("out of music folder buckets");
    }
    Ok(format!("F{nxt:02X}"))
}

fn normalize_ext(ext: &str) -> String {
    let ext = ext.trim();
    if ext.is_empty() {
        return ".mp3".to_string();
    }
    if ext.starts_with('.') {
        ext.to_lowercase()
    } else {
        format!(".{}", ext.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_is_unique_and_hex() {
        let mut existing = HashSet::new();
        for _ in 0..50 {
            let name = generate_filename("mp3", &existing).unwrap();
            assert!(name.ends_with(".mp3"));
            let stem = &name[..name.len() - 4];
            assert_eq!(stem.len(), 4);
            assert!(stem.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(!existing.contains(&name));
            existing.insert(name);
        }
    }

    #[test]
    fn filename_honors_existing_set() {
        let existing: HashSet<String> = ["ABCD.mp3".into(), "1234.mp3".into()].into_iter().collect();
        for _ in 0..100 {
            let name = generate_filename(".MP3", &existing).unwrap();
            assert!(!existing.contains(&name));
            assert!(name.ends_with(".mp3"));
        }
    }

    #[test]
    fn filename_defaults_extension() {
        let name = generate_filename("", &HashSet::new()).unwrap();
        assert!(name.ends_with(".mp3"));
    }

    #[test]
    fn next_folder_from_empty_is_f00() {
        assert_eq!(next_folder_name(&HashSet::new()).unwrap(), "F00");
    }

    #[test]
    fn next_folder_after_existing() {
        let set: HashSet<String> = ["F00".into(), "F87".into(), "F0A".into()].into_iter().collect();
        assert_eq!(next_folder_name(&set).unwrap(), "F88");
    }

    #[test]
    fn next_folder_ignores_unrelated_entries() {
        let set: HashSet<String> = ["F88".into(), "other".into(), "XYZ".into()].into_iter().collect();
        assert_eq!(next_folder_name(&set).unwrap(), "F89");
    }

    #[test]
    fn next_folder_runs_out_of_buckets() {
        let set: HashSet<String> = ["FFF".into()].into_iter().collect();
        assert!(next_folder_name(&set).is_err());
    }
}
