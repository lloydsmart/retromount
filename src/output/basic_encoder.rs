use crate::core::content::{Content, GamePart};
use crate::output::encode::{EncodedFile, OutputEncoder};

pub struct BasicEncoder;

impl BasicEncoder {
    pub fn new() -> Self {
        Self
    }

    fn file_name_for(&self, content: &Content) -> String {
        match content {
            Content::Bytes(bytes) => format!("{}.bin", bytes.id),
            Content::Rom(rom) => rom.file_name.clone(),
            Content::Disc(disc) => format!("{} (Disc {}).cue", disc.title, disc.disc_number),
            Content::Game(game) => match game.parts.as_slice() {
                [GamePart::Rom(rom)] => rom.file_name.clone(),
                [GamePart::Disc(disc)] => format!("{} (Disc {}).cue", game.title, disc.disc_number),
                _ => game.title.clone(),
            },
            Content::Text(text) => normalize_text_name(text.id.0.as_ref()),
        }
    }

    fn size_for(&self, content: &Content) -> u64 {
        match content {
            Content::Bytes(bytes) => bytes.size,
            Content::Rom(rom) => rom.size,
            Content::Disc(_) => 0,
            Content::Game(game) => match game.parts.as_slice() {
                [GamePart::Rom(rom)] => rom.size,
                [GamePart::Disc(_)] => 0,
                _ => 0,
            },
            Content::Text(text) => text.size,
        }
    }
}

fn normalize_text_name(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    match normalized.rsplit_once('.') {
        Some((base, _)) => format!("{base}.txt"),
        None => format!("{normalized}.txt"),
    }
}

impl OutputEncoder for BasicEncoder {
    fn can_encode(&self, _content: &Content) -> bool {
        true
    }

    fn encode(&self, content: &Content) -> Result<EncodedFile, std::io::Error> {
        Ok(EncodedFile {
            name: self.file_name_for(content),
            size: self.size_for(content),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, Content, ContentId, DiscContent, DiscPart, GameContent, GamePart, RomContent,
        RomPart, TextContent,
    };
    use crate::core::source::SourceRef;

    #[test]
    fn encodes_bytes_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Bytes(BytesContent {
            id: ContentId::new("bios"),
            source: SourceRef::new("file:/roms/bios"),
            size: 512,
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "bios.bin");
        assert_eq!(encoded.size, 512);
    }

    #[test]
    fn encodes_rom_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Rom(RomContent {
            id: ContentId::new("mario-world"),
            source: SourceRef::new("zip:/roms/snes.zip#smw.sfc"),
            file_name: "Super Mario World.sfc".to_string(),
            size: 4096,
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "Super Mario World.sfc");
        assert_eq!(encoded.size, 4096);
    }

    #[test]
    fn encodes_disc_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Disc(DiscContent {
            id: ContentId::new("ff7-disc1"),
            source: SourceRef::new("cue:/roms/ff7.cue"),
            title: "Final Fantasy VII".to_string(),
            disc_number: 1,
            consumed_sources: vec![SourceRef::new("cue:/roms/ff7.bin")],
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "Final Fantasy VII (Disc 1).cue");
        assert_eq!(encoded.size, 0);
    }

    #[test]
    fn encodes_single_rom_game_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Game(GameContent {
            id: ContentId::new("smw"),
            source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            title: "Super Mario World".to_string(),
            parts: vec![GamePart::Rom(RomPart {
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                file_name: "Super Mario World.sfc".to_string(),
                size: 4096,
            })],
            consumed_sources: vec![],
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "Super Mario World.sfc");
        assert_eq!(encoded.size, 4096);
    }

    #[test]
    fn encodes_single_disc_game_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Game(GameContent {
            id: ContentId::new("mgs"),
            source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
            title: "Metal Gear Solid".to_string(),
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/mgs-disc1.bin")],
            })],
            consumed_sources: vec![SourceRef::new("cue:/roms/mgs-disc1.bin")],
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "Metal Gear Solid (Disc 1).cue");
        assert_eq!(encoded.size, 0);
    }

    #[test]
    fn encodes_text_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Text(TextContent {
            id: ContentId::new("readme"),
            source: SourceRef::new("file:/roms/readme"),
            size: 128,
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "readme.txt");
        assert_eq!(encoded.size, 128);
    }

    #[test]
    fn preserves_relative_path_for_text_content() {
        let encoder = BasicEncoder::new();
        let content = Content::Text(TextContent {
            id: ContentId::new("mixed/notes"),
            source: SourceRef::new("mixed/notes.txt"),
            size: 10,
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "mixed/notes.txt");
        assert_eq!(encoded.size, 10);
    }

    #[test]
    fn normalizes_text_extension_from_id_path() {
        let encoder = BasicEncoder::new();
        let content = Content::Text(TextContent {
            id: ContentId::new("roms/snes/game"),
            source: SourceRef::new("roms/snes/game.nfo"),
            size: 19,
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "roms/snes/game.txt");
        assert_eq!(encoded.size, 19);
    }
}
