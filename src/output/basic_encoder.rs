use crate::core::content::Content;
use crate::output::encode::{EncodedFile, OutputEncoder};

#[derive(Debug, Default)]
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
            Content::Text(text) => format!("{}.txt", text.id),
        }
    }

    fn size_for(&self, content: &Content) -> u64 {
        match content {
            Content::Bytes(bytes) => bytes.size,
            Content::Rom(rom) => rom.size,
            Content::Disc(_) => 0,
            Content::Text(text) => text.size,
        }
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
        BytesContent, Content, ContentId, DiscContent, RomContent, TextContent,
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
        });

        let encoded = encoder.encode(&content).unwrap();
        assert_eq!(encoded.name, "Final Fantasy VII (Disc 1).cue");
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
}
