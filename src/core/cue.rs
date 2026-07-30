use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueFileEntry {
    pub path: PathBuf,
    pub file_type: String,
    pub tracks: Vec<CueTrackEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueTrackEntry {
    pub number: u8,
    pub mode: String,
    pub indexes: Vec<CueIndexEntry>,
    pub pregap_sectors: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CueIndexEntry {
    pub number: u8,
    pub sector: u64,
}

pub fn parse_cue(cue_text: &str) -> io::Result<Vec<CueFileEntry>> {
    let mut files = Vec::new();
    let mut current_file: Option<CueFileEntry> = None;

    for (line_index, line) in cue_text.lines().enumerate() {
        let trimmed = line.trim_start();

        if starts_with_keyword(trimmed, "FILE") {
            let (path, file_type) = parse_file_line(trimmed)
                .ok_or_else(|| cue_error(line_index, "invalid FILE directive"))?;
            if let Some(file) = current_file.take() {
                files.push(file);
            }

            current_file = Some(CueFileEntry {
                path: PathBuf::from(path),
                file_type,
                tracks: Vec::new(),
            });

            continue;
        }

        if starts_with_keyword(trimmed, "TRACK") {
            let track = parse_track_line(trimmed)
                .ok_or_else(|| cue_error(line_index, "invalid TRACK directive"))?;
            let file = current_file
                .as_mut()
                .ok_or_else(|| cue_error(line_index, "TRACK appears before FILE"))?;
            file.tracks.push(track);
            continue;
        }

        if starts_with_keyword(trimmed, "INDEX") {
            let index = parse_index_line(trimmed)
                .ok_or_else(|| cue_error(line_index, "invalid INDEX directive"))?;
            current_track_mut(&mut current_file, line_index)?
                .indexes
                .push(index);
            continue;
        }

        if starts_with_keyword(trimmed, "PREGAP") {
            let pregap = parse_pregap_line(trimmed)
                .ok_or_else(|| cue_error(line_index, "invalid PREGAP directive"))?;
            current_track_mut(&mut current_file, line_index)?.pregap_sectors = Some(pregap);
        }
    }

    if let Some(file) = current_file {
        files.push(file);
    }

    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "CUE contains no FILE directives",
        ));
    }

    Ok(files)
}

fn parse_file_line(line: &str) -> Option<(String, String)> {
    let rest = line.get(4..)?.trim_start();

    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        let path = quoted[..end].to_string();
        let file_type = quoted[end + 1..].split_whitespace().next()?.to_string();
        return Some((path, file_type));
    }

    let mut parts = rest.split_whitespace();
    Some((parts.next()?.to_string(), parts.next()?.to_string()))
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

    Some(CueTrackEntry {
        number,
        mode,
        indexes: Vec::new(),
        pregap_sectors: None,
    })
}

fn parse_index_line(line: &str) -> Option<CueIndexEntry> {
    let mut parts = line.split_whitespace();
    let _index_kw = parts.next()?;
    let number = parts.next()?.parse::<u8>().ok()?;
    let sector = parse_msf(parts.next()?)?;
    Some(CueIndexEntry { number, sector })
}

fn parse_pregap_line(line: &str) -> Option<u64> {
    let mut parts = line.split_whitespace();
    let _pregap_kw = parts.next()?;
    parse_msf(parts.next()?)
}

fn parse_msf(value: &str) -> Option<u64> {
    let mut parts = value.split(':');
    let minutes = parts.next()?.parse::<u64>().ok()?;
    let seconds = parts.next()?.parse::<u64>().ok()?;
    let frames = parts.next()?.parse::<u64>().ok()?;

    if parts.next().is_some() || seconds >= 60 || frames >= 75 {
        return None;
    }

    minutes
        .checked_mul(60)?
        .checked_add(seconds)?
        .checked_mul(75)?
        .checked_add(frames)
}

fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    line.get(..keyword.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
        && line
            .get(keyword.len()..)
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn current_track_mut(
    current_file: &mut Option<CueFileEntry>,
    line_index: usize,
) -> io::Result<&mut CueTrackEntry> {
    current_file
        .as_mut()
        .and_then(|file| file.tracks.last_mut())
        .ok_or_else(|| cue_error(line_index, "track directive appears before TRACK"))
}

fn cue_error(line_index: usize, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("CUE line {}: {message}", line_index + 1),
    )
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

        let parsed = parse_cue(cue).unwrap();

        assert_eq!(parsed.len(), 2);

        assert_eq!(parsed[0].path, PathBuf::from("track01.bin"));
        assert_eq!(parsed[0].file_type, "BINARY");
        assert_eq!(parsed[0].tracks.len(), 1);
        assert_eq!(parsed[0].tracks[0].number, 1);
        assert_eq!(parsed[0].tracks[0].mode, "MODE1/2352");
        assert_eq!(
            parsed[0].tracks[0].indexes,
            [CueIndexEntry {
                number: 1,
                sector: 0
            }]
        );

        assert_eq!(parsed[1].path, PathBuf::from("track02.bin"));
        assert_eq!(parsed[1].tracks.len(), 1);
        assert_eq!(parsed[1].tracks[0].number, 2);
        assert_eq!(parsed[1].tracks[0].mode, "AUDIO");
    }

    #[test]
    fn parses_indexes_and_pregaps_as_sectors() {
        let cue = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 00 00:00:00
    INDEX 01 00:02:00
  TRACK 02 AUDIO
    PREGAP 00:01:00
    INDEX 01 10:00:00
"#;

        let parsed = parse_cue(cue).unwrap();

        assert_eq!(parsed[0].tracks[0].indexes[1].sector, 150);
        assert_eq!(parsed[0].tracks[1].pregap_sectors, Some(75));
        assert_eq!(parsed[0].tracks[1].indexes[0].sector, 45_000);
    }

    #[test]
    fn rejects_malformed_layout_directives() {
        let error = parse_cue("TRACK 01 AUDIO").unwrap_err();
        assert!(error.to_string().contains("before FILE"));

        let error = parse_cue("FILE \"game.bin\" BINARY\n INDEX 01 00:00:00").unwrap_err();
        assert!(error.to_string().contains("before TRACK"));

        let error =
            parse_cue("FILE \"game.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:99:00").unwrap_err();
        assert!(error.to_string().contains("invalid INDEX"));
    }
}
