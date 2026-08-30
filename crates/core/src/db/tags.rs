//! Audio metadata extraction via `lofty` (mp3 / m4a / flac / ...).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};

use super::Track;

/// read tags + audio props from file - relaxed so bad tags don't block adding
pub fn read_track(path: &Path) -> Result<Track> {
    let file_size = fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .len() as i64;

    let tagged = Probe::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .options(ParseOptions::new().parsing_mode(ParsingMode::Relaxed))
        .read();
    let tagged = match tagged {
        Ok(t) => t,
        Err(_) => Probe::open(path)
            .with_context(|| format!("opening {}", path.display()))?
            .options(ParseOptions::new().read_tags(false))
            .read()
            .with_context(|| format!("reading audio info from {}", path.display()))?,
    };

    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let props = tagged.properties();
    // Fall back to the filename when no title tag is present.
    let filename_title = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned());

    Ok(Track {
        title: tag
            .and_then(|t| t.title())
            .map(|s| s.to_string())
            .or(filename_title),
        artist: tag.and_then(|t| t.artist()).map(|s| s.to_string()),
        album: tag.and_then(|t| t.album()).map(|s| s.to_string()),
        album_artist: tag.and_then(|t| t.get_string(&ItemKey::AlbumArtist)).map(|s| s.to_string()),
        genre: tag.and_then(|t| t.genre()).map(|s| s.to_string()),
        track_number: tag.and_then(|t| t.track()).map(|n| n as i64),
        disc_number: tag.and_then(|t| t.disk()).map(|n| n as i64),
        year: tag.and_then(|t| t.year()).map(|n| n as i64),
        duration_s: Some(props.duration().as_secs_f64()),
        bitrate: props.audio_bitrate().map(|b| b as i64),
        sample_rate: props.sample_rate().map(|s| s as f64),
        location: String::new(),
        file_size,
    })
}
