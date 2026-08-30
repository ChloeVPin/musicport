//! nice filenames for export: "Artist - Title.mp3" with (1) if dup

use std::collections::HashSet;
use std::path::Path;

fn clean_component(value: Option<&str>) -> String {
    let cleaned: String = value
        .unwrap_or("")
        .trim()
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    if cleaned.is_empty() {
        "Unknown".to_string()
    } else {
        cleaned
    }
}

/// `"Artist - Title.ext"`, falling back to "Unknown" for missing parts.
pub(crate) fn safe_name(artist: Option<&str>, title: Option<&str>, ext: &str) -> String {
    format!(
        "{} - {}{}",
        clean_component(artist),
        clean_component(title),
        ext
    )
}

/// make base unique in used, add (1) (2) if needed
pub(crate) fn unique_name(base: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = base.to_string();
    let mut n = 1u32;
    while used.contains(&candidate) {
        let path = Path::new(base);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let suffix = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        candidate = format!("{stem} ({n}){suffix}");
        n += 1;
    }
    used.insert(candidate.clone());
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_component_strips_and_defaults() {
        assert_eq!(clean_component(None), "Unknown");
        assert_eq!(clean_component(Some("  ")), "Unknown");
        assert_eq!(clean_component(Some("A/B:C")), "A_B_C");
        assert_eq!(clean_component(Some(" Juice WRLD ")), "Juice WRLD");
    }

    #[test]
    fn safe_name_format() {
        assert_eq!(
            safe_name(Some("Artist"), Some("Title"), ".mp3"),
            "Artist - Title.mp3"
        );
        assert_eq!(safe_name(None, None, ".flac"), "Unknown - Unknown.flac");
    }

    #[test]
    fn unique_name_appends_counters() {
        let mut used = HashSet::new();
        let a = unique_name("Artist - Title.mp3", &mut used);
        let b = unique_name("Artist - Title.mp3", &mut used);
        let c = unique_name("Artist - Title.mp3", &mut used);
        assert_eq!(a, "Artist - Title.mp3");
        assert_eq!(b, "Artist - Title (1).mp3");
        assert_eq!(c, "Artist - Title (2).mp3");
        assert_eq!(used.len(), 3);
    }
}