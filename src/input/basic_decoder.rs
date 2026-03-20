use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::content::{
    BytesContent, Content, ContentId, DiscContent, RomContent, TextContent,
};
use crate::core::cue::parse_cue;
use crate::core::reader::Reader;
use crate::core::source::{SourceObject, SourceRef};
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;
use crate::readers::dir_reader::DirReader;
use crate::readers::zip_reader::ZipReader;

#[derive(Debug, Default)]
pub struct BasicInputDecoder;

impl BasicInputDecoder {
    pub fn new() -> Self {
        Self
    }
}

impl InputDecoder for BasicInputDecoder {
    fn supports(&self, identity: &InputIdentity) -> bool {
        matches!(
            identity,
            InputIdentity::File
                | InputIdentity::Text
                | InputIdentity::DiscImage
                | InputIdentity::Unknown
        )
    }

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<Content>, io::Error> {
        let id = ContentId::new(object.name.clone());
        let size = content_size_for(object)?;

        let content = match identity {
            InputIdentity::Text => Content::Text(TextContent {
                id: ContentId::new(path_without_extension(&object.name)),
                source: object.source.clone(),
                size,
            }),
            InputIdentity::DiscImage => {
                let raw_name = file_stem_or_name_from_object(object);
                let (title, disc_number) = parse_disc_info_from_name(&raw_name);

                Content::Disc(DiscContent {
                    id,
                    source: object.source.clone(),
                    title,
                    disc_number,
                    consumed_sources: consumed_sources_for_disc_image(object)?,
                })
            }
            InputIdentity::File => {
                if looks_like_rom_name(&object.name) {
                    Content::Rom(RomContent {
                        id,
                        source: object.source.clone(),
                        file_name: object.name.clone(),
                        size,
                    })
                } else {
                    Content::Bytes(BytesContent {
                        id,
                        source: object.source.clone(),
                        size,
                    })
                }
            }
            InputIdentity::Unknown => Content::Bytes(BytesContent {
                id,
                source: object.source.clone(),
                size,
            }),
            InputIdentity::Directory | InputIdentity::Archive => {
                return Ok(Vec::new());
            }
        };

        Ok(vec![content])
    }
}

fn content_size_for(object: &SourceObject) -> Result<u64, io::Error> {
    if let Some((archive_path, entry_name)) = parse_zip_source(object) {
        let reader = ZipReader::open(Path::new(&archive_path), &entry_name)?;
        return Ok(reader.len());
    }

    let path = Path::new(object.source.0.as_ref());
    Ok(fs::metadata(path)?.len())
}

fn parse_zip_source(object: &SourceObject) -> Option<(String, String)> {
    let source = object.source.0.as_ref();
    let remainder = source.strip_prefix("zip:")?;
    let (archive_path, entry_name) = remainder.split_once('#')?;

    Some((archive_path.to_string(), entry_name.to_string()))
}

fn consumed_sources_for_disc_image(object: &SourceObject) -> Result<Vec<SourceRef>, io::Error> {
    let cue_text = read_text_from_source(object)?;
    let parsed = parse_cue(&cue_text);

    let mut consumed = Vec::new();

    for file_entry in parsed {
        let source = resolve_relative_source(object, &file_entry.path);
        consumed.push(source);
    }

    Ok(consumed)
}

fn read_text_from_source(object: &SourceObject) -> Result<String, io::Error> {
    if let Some((archive_path, entry_name)) = parse_zip_source(object) {
        let mut reader = ZipReader::open(Path::new(&archive_path), &entry_name)?;
        let mut bytes = vec![0; reader.len() as usize];
        let bytes_read = reader.read_at(0, &mut bytes)?;
        bytes.truncate(bytes_read);

        return String::from_utf8(bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err));
    }

    let path = Path::new(object.source.0.as_ref());
    let mut reader = DirReader::open(path)?;
    let mut bytes = vec![0; reader.len() as usize];
    let bytes_read = reader.read_at(0, &mut bytes)?;
    bytes.truncate(bytes_read);

    String::from_utf8(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn resolve_relative_source(object: &SourceObject, referenced_path: &Path) -> SourceRef {
    if let Some((archive_path, entry_name)) = parse_zip_source(object) {
        let entry_path = Path::new(&entry_name);
        let base_dir = entry_path.parent().unwrap_or_else(|| Path::new(""));
        let resolved = normalize_virtual_path(base_dir.join(referenced_path));

        return SourceRef::new(format!("zip:{archive_path}#{resolved}"));
    }

    let cue_path = Path::new(object.source.0.as_ref());
    let base_dir = cue_path.parent().unwrap_or_else(|| Path::new("."));
    let resolved = base_dir.join(referenced_path);

    SourceRef::new(resolved.to_string_lossy().into_owned())
}

fn normalize_virtual_path(path: PathBuf) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn looks_like_rom_name(name: &str) -> bool {
    matches!(
        Path::new(name).extension().and_then(|ext| ext.to_str()),
        Some(ext)
            if ext.eq_ignore_ascii_case("rom")
                || ext.eq_ignore_ascii_case("bin")
                || ext.eq_ignore_ascii_case("sfc")
                || ext.eq_ignore_ascii_case("smc")
                || ext.eq_ignore_ascii_case("nes")
                || ext.eq_ignore_ascii_case("gb")
                || ext.eq_ignore_ascii_case("gba")
                || ext.eq_ignore_ascii_case("md")
                || ext.eq_ignore_ascii_case("gen")
    )
}

fn file_stem_or_name_from_object(object: &SourceObject) -> String {
    Path::new(&object.name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(&object.name)
        .to_string()
}

fn path_without_extension(name: &str) -> String {
    let path = Path::new(name);

    match (
        path.parent(),
        path.file_stem().and_then(|stem| stem.to_str()),
    ) {
        (Some(parent), Some(stem)) if !parent.as_os_str().is_empty() => {
            parent.join(stem).to_string_lossy().into_owned()
        }
        (_, Some(stem)) => stem.to_string(),
        _ => name.to_string(),
    }
}

fn parse_disc_info_from_name(name: &str) -> (String, u32) {
    let lower = name.to_lowercase();

    // patterns like:
    // game_disc1
    // game-disc2
    // game cd1
    // game (disc 2)
    let patterns = ["disc", "cd", "disk", "vol"];

    for pattern in patterns {
        if let Some(idx) = lower.rfind(pattern) {
            let after = &lower[idx + pattern.len()..];

            let number: String = after
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();

            if let Ok(num) = number.parse::<u32>() {
                let base = name[..idx]
                    .trim_end_matches(|c: char| {
                        c == '_' || c == '-' || c == ' ' || c == '(' || c == '['
                    })
                    .trim()
                    .to_string();

                return (base, num);
            }
        }
    }

    (name.to_string(), 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::core::source::{SourceObject, SourceRef};

    #[test]
    fn decodes_text_file_as_text_content() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("readme.txt");
        fs::write(&path, b"hello").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(path.to_string_lossy().into_owned()),
            name: "readme.txt".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::Text).unwrap();
        assert_eq!(content.len(), 1);
        assert!(matches!(content[0], Content::Text(_)));
    }

    #[test]
    fn decodes_rom_file_as_rom_content() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("game.sfc");
        fs::write(&path, b"romdata").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(path.to_string_lossy().into_owned()),
            name: "game.sfc".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::File).unwrap();
        assert_eq!(content.len(), 1);
        assert!(matches!(content[0], Content::Rom(_)));
    }

    #[test]
    fn decodes_zip_source_with_entry_size() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = tempfile::tempdir().unwrap();
        let zip_path = temp_dir.path().join("test.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();

            zip.start_file("roms/sonic.bin", options).unwrap();
            zip.write_all(b"romdata").unwrap();
            zip.finish().unwrap();
        }

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(format!("zip:{}#roms/sonic.bin", zip_path.to_string_lossy())),
            name: "sonic.bin".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::File).unwrap();
        assert_eq!(content.len(), 1);

        match &content[0] {
            Content::Rom(rom) => assert_eq!(rom.size, 7),
            other => panic!("expected Rom content, got {other:?}"),
        }
    }

    #[test]
    fn disc_content_tracks_consumed_sources_for_filesystem_cue() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cue_path = temp_dir.path().join("game.cue");
        let bin_path = temp_dir.path().join("game.bin");

        fs::write(&bin_path, b"discdata").unwrap();
        fs::write(
            &cue_path,
            r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#,
        )
        .unwrap();

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(cue_path.to_string_lossy().into_owned()),
            name: "game.cue".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::DiscImage).unwrap();
        assert_eq!(content.len(), 1);

        match &content[0] {
            Content::Disc(disc) => {
                assert_eq!(disc.consumed_sources.len(), 1);
                assert_eq!(
                    disc.consumed_sources[0].to_string(),
                    bin_path.to_string_lossy()
                );
            }
            other => panic!("expected Disc content, got {other:?}"),
        }
    }

    #[test]
    fn disc_content_tracks_consumed_sources_for_zip_cue() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = tempfile::tempdir().unwrap();
        let zip_path = temp_dir.path().join("test.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();

            zip.start_file("ps1/game.bin", options).unwrap();
            zip.write_all(b"discdata").unwrap();

            zip.start_file("ps1/game.cue", options).unwrap();
            zip.write_all(
                br#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#,
            )
            .unwrap();

            zip.finish().unwrap();
        }

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(format!("zip:{}#ps1/game.cue", zip_path.to_string_lossy())),
            name: "ps1/game.cue".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::DiscImage).unwrap();
        assert_eq!(content.len(), 1);

        match &content[0] {
            Content::Disc(disc) => {
                assert_eq!(disc.consumed_sources.len(), 1);
                assert_eq!(
                    disc.consumed_sources[0].to_string(),
                    format!("zip:{}#ps1/game.bin", zip_path.to_string_lossy())
                );
            }
            other => panic!("expected Disc content, got {other:?}"),
        }
    }

    #[test]
    fn parses_disc_suffix_from_filename() {
        assert_eq!(
            parse_disc_info_from_name("game_disc1"),
            ("game".to_string(), 1)
        );
        assert_eq!(
            parse_disc_info_from_name("game-disc2"),
            ("game".to_string(), 2)
        );
        assert_eq!(
            parse_disc_info_from_name("game cd3"),
            ("game".to_string(), 3)
        );
        assert_eq!(
            parse_disc_info_from_name("game (disc 4)"),
            ("game".to_string(), 4)
        );
    }

    #[test]
    fn parses_disc_suffix_with_parentheses_cleanly() {
        assert_eq!(
            parse_disc_info_from_name("Game (Disc 1)"),
            ("Game".to_string(), 1)
        );
    }

    #[test]
    fn defaults_to_disc_one_when_no_disc_suffix_exists() {
        assert_eq!(parse_disc_info_from_name("game"), ("game".to_string(), 1));
    }

    #[test]
    fn decodes_text_file_with_relative_path_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("notes.txt");
        fs::write(&path, b"hello").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(path.to_string_lossy().into_owned()),
            name: "mixed/notes.txt".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::Text).unwrap();

        match &content[0] {
            Content::Text(text) => assert_eq!(text.id.to_string(), "mixed/notes"),
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn decodes_text_file_with_normalized_relative_path_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("game.nfo");
        fs::write(&path, b"hello").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new(path.to_string_lossy().into_owned()),
            name: "roms/snes/game.nfo".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::Text).unwrap();

        match &content[0] {
            Content::Text(text) => assert_eq!(text.id.to_string(), "roms/snes/game"),
            other => panic!("expected text content, got {other:?}"),
        }
    }
}
