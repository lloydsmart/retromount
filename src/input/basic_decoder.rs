use std::fs;
use std::io;
use std::path::Path;

use crate::core::content::{
    BytesContent, Content, ContentId, DiscContent, RomContent, TextContent,
};
use crate::core::source::SourceObject;
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
    ) -> Result<Vec<Content>, io::Error> {
        let id = ContentId::new(object.name.clone());
        let size = content_size_for(object)?;

        let content = match identity {
            InputIdentity::Text => Content::Text(TextContent {
                id: ContentId::new(file_stem_or_name_from_object(object)),
                source: object.source.clone(),
                size,
            }),
            InputIdentity::DiscImage => Content::Disc(DiscContent {
                id,
                source: object.source.clone(),
                title: file_stem_or_name_from_object(object),
                disc_number: 1,
            }),
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
    if is_zip_source(object) {
        return Ok(0);
    }

    let path = Path::new(object.source.0.as_ref());
    Ok(fs::metadata(path)?.len())
}

fn is_zip_source(object: &SourceObject) -> bool {
    object.source.0.starts_with("zip:")
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
    fn decodes_zip_source_without_filesystem_metadata() {
        let decoder = BasicInputDecoder::new();
        let object = SourceObject {
            source: SourceRef::new("zip:/tmp/test.zip#roms/sonic.bin"),
            name: "sonic.bin".to_string(),
        };

        let content = decoder.decode(&object, &InputIdentity::File).unwrap();
        assert_eq!(content.len(), 1);
        assert!(matches!(content[0], Content::Rom(_)));
    }
}
