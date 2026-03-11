use std::path::PathBuf;

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
}
