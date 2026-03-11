use std::path::{Path, PathBuf};

use crate::core::disc::Disc;
use crate::core::track::{Track, TrackSource, TrackType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueFileEntry {
    pub path: PathBuf,
    pub tracks: Vec<CueTrackEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueTrackEntry {
    pub number: u8,
    pub mode: String,
}

pub fn parse_cue(cue_text: &str) -> Vec<CueFileEntry> {
    let mut files = Vec::new();
    let mut current_file: Option<CueFileEntry> = None;

    for line in cue_text.lines() {
        let trimmed = line.trim_start();

        if let Some(path) = parse_file_line(trimmed) {
            if let Some(file) = current_file.take() {
                files.push(file);
            }

            current_file = Some(CueFileEntry {
                path: PathBuf::from(path),
                tracks: Vec::new(),
            });

            continue;
        }

        if let Some(track) = parse_track_line(trimmed) {
            if let Some(file) = current_file.as_mut() {
                file.tracks.push(track);
            }
        }
    }

    if let Some(file) = current_file {
        files.push(file);
    }

    files
}

pub fn cue_to_disc(cue_text: &str, cue_dir: &Path, disc_number: u32) -> Disc {
    let parsed = parse_cue(cue_text);
    let mut tracks = Vec::new();

    for file_entry in parsed {
        let source_path = cue_dir.join(&file_entry.path);

        for track_entry in file_entry.tracks {
            let (kind, sector_size) = cue_track_mode_to_model(&track_entry.mode);

            tracks.push(Track {
                number: u32::from(track_entry.number),
                kind,
                size: 0,
                sector_size,
                source: TrackSource::File(source_path.clone()),
            });
        }
    }

    Disc {
        number: disc_number,
        tracks,
    }
}

fn cue_track_mode_to_model(mode: &str) -> (TrackType, u32) {
    let upper = mode.to_ascii_uppercase();

    if upper == "AUDIO" {
        return (TrackType::Audio, 2352);
    }

    if let Some(sector_size) = upper.split('/').nth(1).and_then(|s| s.parse::<u32>().ok()) {
        return (TrackType::Data, sector_size);
    }

    (TrackType::Data, 2048)
}

fn parse_file_line(line: &str) -> Option<String> {
    if !line
        .get(0..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("FILE"))
    {
        return None;
    }

    let rest = line.get(4..)?.trim_start();

    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        return Some(quoted[..end].to_string());
    }

    Some(rest.split_whitespace().next()?.to_string())
}

fn parse_track_line(line: &str) -> Option<CueTrackEntry> {
    if !line
        .get(0..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("TRACK"))
    {
        return None;
    }

    let mut parts = line.split_whitespace();
    let _track_kw = parts.next()?;
    let number = parts.next()?.parse::<u8>().ok()?;
    let mode = parts.next()?.to_string();

    Some(CueTrackEntry { number, mode })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_and_track_entries() {
        let cue = r#"
FILE "track01.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
FILE "track02.bin" BINARY
  TRACK 02 AUDIO
    INDEX 01 00:00:00
"#;

        let parsed = parse_cue(cue);

        assert_eq!(parsed.len(), 2);

        assert_eq!(parsed[0].path, PathBuf::from("track01.bin"));
        assert_eq!(parsed[0].tracks.len(), 1);
        assert_eq!(parsed[0].tracks[0].number, 1);
        assert_eq!(parsed[0].tracks[0].mode, "MODE1/2352");

        assert_eq!(parsed[1].path, PathBuf::from("track02.bin"));
        assert_eq!(parsed[1].tracks.len(), 1);
        assert_eq!(parsed[1].tracks[0].number, 2);
        assert_eq!(parsed[1].tracks[0].mode, "AUDIO");
    }

    #[test]
    fn converts_cue_to_disc_model() {
        let cue = r#"
FILE "track01.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
FILE "track02.bin" BINARY
  TRACK 02 AUDIO
    INDEX 01 00:00:00
"#;

        let disc = cue_to_disc(cue, Path::new("/roms/ps1/game"), 1);

        assert_eq!(disc.number, 1);
        assert_eq!(disc.tracks.len(), 2);

        assert_eq!(disc.tracks[0].number, 1);
        assert_eq!(disc.tracks[0].kind, TrackType::Data);
        assert_eq!(disc.tracks[0].sector_size, 2352);
        assert_eq!(
            disc.tracks[0].source,
            TrackSource::File(PathBuf::from("/roms/ps1/game/track01.bin"))
        );

        assert_eq!(disc.tracks[1].number, 2);
        assert_eq!(disc.tracks[1].kind, TrackType::Audio);
        assert_eq!(disc.tracks[1].sector_size, 2352);
        assert_eq!(
            disc.tracks[1].source,
            TrackSource::File(PathBuf::from("/roms/ps1/game/track02.bin"))
        );
    }

    #[test]
    fn defaults_unknown_data_mode_to_2048_sector_size() {
        let cue = r#"
FILE "track01.bin" BINARY
  TRACK 01 MODE1
    INDEX 01 00:00:00
"#;

        let disc = cue_to_disc(cue, Path::new("."), 1);

        assert_eq!(disc.tracks.len(), 1);
        assert_eq!(disc.tracks[0].kind, TrackType::Data);
        assert_eq!(disc.tracks[0].sector_size, 2048);
    }
}
