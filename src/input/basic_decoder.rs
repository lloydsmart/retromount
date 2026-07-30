use std::io;
use std::path::Path;

use crate::core::content::{
    BytesContent, ContentId, DecodedContent, DecodedDiscContent, DecodedRomContent, TextContent,
};
use crate::core::cue::parse_cue;
use crate::core::source::{SourceObject, SourceRef};
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;

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
    ) -> Result<Vec<DecodedContent>, io::Error> {
        let id = ContentId::new(object.name.clone());
        let size = content_size_for(object)?;

        let content = match identity {
            InputIdentity::Text => DecodedContent::Text(TextContent {
                id: ContentId::new(path_without_extension(&object.name)),
                source: object.source.clone(),
                size,
            }),
            InputIdentity::DiscImage => {
                let raw_name = file_stem_or_name_from_object(object);
                let (title, disc_number) = parse_disc_info_from_name(&raw_name);

                DecodedContent::Disc(DecodedDiscContent {
                    id,
                    source: object.source.clone(),
                    title,
                    disc_number,
                    consumed_sources: consumed_sources_for_disc_image(object)?,
                    cd_disc: None,
                    logical_disc: None,
                })
            }
            InputIdentity::File => {
                if looks_like_rom_name(&object.name) {
                    DecodedContent::Rom(DecodedRomContent {
                        id,
                        source: object.source.clone(),
                        size,
                    })
                } else {
                    DecodedContent::Bytes(BytesContent {
                        id,
                        source: object.source.clone(),
                        size,
                    })
                }
            }
            InputIdentity::Unknown => DecodedContent::Bytes(BytesContent {
                id,
                source: object.source.clone(),
                size,
            }),
            InputIdentity::Directory
            | InputIdentity::Archive
            | InputIdentity::ChdDisc
            | InputIdentity::IsoDisc => {
                return Ok(Vec::new());
            }
        };

        Ok(vec![content])
    }
}

fn content_size_for(object: &SourceObject) -> Result<u64, io::Error> {
    Ok(object.content.size)
}

fn consumed_sources_for_disc_image(object: &SourceObject) -> Result<Vec<SourceRef>, io::Error> {
    let cue_text = read_text_from_source(object)?;
    let parsed = parse_cue(&cue_text)?;

    let mut consumed = Vec::new();

    for file_entry in parsed {
        let source = resolve_relative_source(object, &file_entry.path);
        consumed.push(source);
    }

    Ok(consumed)
}

fn read_text_from_source(object: &SourceObject) -> Result<String, io::Error> {
    let mut reader = object.content.open()?;
    let mut bytes = vec![0; reader.len() as usize];
    let bytes_read = reader.read_at(0, &mut bytes)?;
    bytes.truncate(bytes_read);

    String::from_utf8(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn resolve_relative_source(object: &SourceObject, referenced_path: &Path) -> SourceRef {
    object.resolve_relative(referenced_path)
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

    let stem = match path.file_stem().and_then(|stem| stem.to_str()) {
        Some(stem) => stem,
        None => return name.to_string(),
    };

    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            let parent = parent
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");

            format!("{parent}/{stem}")
        }
        _ => stem.to_string(),
    }
}

fn parse_disc_info_from_name(name: &str) -> (String, u32) {
    let lower = name.to_lowercase();

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

    use crate::core::source::SourceObject;
    use crate::input::file_source::FileInputSource;
    use crate::input::source::InputSource;
    use crate::input::zip_source::ZipInputSource;

    fn filesystem_object(path: &Path, name: &str) -> SourceObject {
        let mut object = FileInputSource::new(path).enumerate().unwrap().remove(0);
        object.name = name.to_string();
        object
    }

    #[test]
    fn decodes_text_file_as_text_content() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("readme.txt");
        fs::write(&path, b"hello").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = filesystem_object(&path, "readme.txt");

        let content = decoder.decode(&object, &InputIdentity::Text).unwrap();
        assert_eq!(content.len(), 1);
        assert!(matches!(content[0], DecodedContent::Text(_)));
    }

    #[test]
    fn decodes_rom_file_as_rom_content() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("game.sfc");
        fs::write(&path, b"romdata").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = filesystem_object(&path, "game.sfc");

        let content = decoder.decode(&object, &InputIdentity::File).unwrap();
        assert_eq!(content.len(), 1);
        assert!(matches!(content[0], DecodedContent::Rom(_)));
    }

    #[test]
    fn leaves_chd_content_for_a_dedicated_decoder() {
        let decoder = BasicInputDecoder::new();

        assert!(!decoder.supports(&InputIdentity::ChdDisc));
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
        let object = ZipInputSource::new(&zip_path)
            .enumerate()
            .unwrap()
            .remove(0);

        let content = decoder.decode(&object, &InputIdentity::File).unwrap();
        assert_eq!(content.len(), 1);

        match &content[0] {
            DecodedContent::Rom(rom) => assert_eq!(rom.size, 7),
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
        let object = filesystem_object(&cue_path, "game.cue");

        let content = decoder.decode(&object, &InputIdentity::DiscImage).unwrap();
        assert_eq!(content.len(), 1);

        match &content[0] {
            DecodedContent::Disc(disc) => {
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
        let object = ZipInputSource::new(&zip_path)
            .enumerate()
            .unwrap()
            .into_iter()
            .find(|object| object.name == "ps1/game.cue")
            .unwrap();

        let content = decoder.decode(&object, &InputIdentity::DiscImage).unwrap();
        assert_eq!(content.len(), 1);

        match &content[0] {
            DecodedContent::Disc(disc) => {
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
        let object = filesystem_object(&path, "mixed/notes.txt");

        let content = decoder.decode(&object, &InputIdentity::Text).unwrap();

        match &content[0] {
            DecodedContent::Text(text) => assert_eq!(text.id.to_string(), "mixed/notes"),
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn decodes_text_file_with_normalized_relative_path_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("game.nfo");
        fs::write(&path, b"hello").unwrap();

        let decoder = BasicInputDecoder::new();
        let object = filesystem_object(&path, "roms/snes/game.nfo");

        let content = decoder.decode(&object, &InputIdentity::Text).unwrap();

        match &content[0] {
            DecodedContent::Text(text) => assert_eq!(text.id.to_string(), "roms/snes/game"),
            other => panic!("expected text content, got {other:?}"),
        }
    }
}
