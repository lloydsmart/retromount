use crate::core::content::{GameContent, GamePart, NormalizedContent};
use crate::output::encode::{EncodedBacking, EncodedFile, OutputEncoder};
use crate::policy::PolicySet;

pub struct BasicEncoder;

impl BasicEncoder {
    pub fn new() -> Self {
        Self
    }

    fn file_name_for(&self, content: &NormalizedContent, policy: &PolicySet) -> String {
        let raw = match content {
            NormalizedContent::Bytes(bytes) => format!("{}.bin", bytes.id),
            NormalizedContent::Game(game) => match game.parts.as_slice() {
                [part] => policy.naming().part_name(game, part),
                _ => policy.naming().game_name(game),
            },
            NormalizedContent::Text(text) => normalize_text_name(text.id.0.as_ref()),
        };

        policy.format_name(&raw)
    }

    fn backing_for(&self, content: &NormalizedContent) -> EncodedBacking {
        match content {
            NormalizedContent::Bytes(bytes) => EncodedBacking::SourceBacked {
                size: bytes.size,
                source: bytes.source.clone(),
            },
            NormalizedContent::Game(game) => match game.parts.as_slice() {
                [part] => self.backing_for_game_part(part),
                _ => EncodedBacking::SourceBacked {
                    size: 0,
                    source: game.source.clone(),
                },
            },
            NormalizedContent::Text(text) => EncodedBacking::SourceBacked {
                size: text.size,
                source: text.source.clone(),
            },
        }
    }

    fn file_name_for_game_part(
        &self,
        game: &GameContent,
        part: &GamePart,
        policy: &PolicySet,
    ) -> String {
        let raw = policy.naming().part_name(game, part);
        policy.format_name(&raw)
    }

    fn backing_for_game_part(&self, part: &GamePart) -> EncodedBacking {
        match part {
            GamePart::Rom(rom) => EncodedBacking::SourceBacked {
                size: rom.size,
                source: rom.source.clone(),
            },
            GamePart::Disc(disc) => EncodedBacking::SourceBacked {
                size: 0,
                source: disc.source.clone(),
            },
        }
    }

    fn playlist_name_for(&self, game: &GameContent, policy: &PolicySet) -> String {
        let raw = policy.naming().playlist_name(game);
        policy.format_name(&raw)
    }

    fn playlist_backing(&self, entries: &[String]) -> EncodedBacking {
        EncodedBacking::Inline((entries.join("\n") + "\n").into_bytes())
    }
}

impl Default for BasicEncoder {
    fn default() -> Self {
        Self::new()
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
    fn can_encode(&self, _content: &NormalizedContent) -> bool {
        true
    }

    fn encode(
        &self,
        content: &NormalizedContent,
        policy: &PolicySet,
    ) -> Result<EncodedFile, std::io::Error> {
        Ok(EncodedFile {
            name: self.file_name_for(content, policy),
            backing: self.backing_for(content),
        })
    }

    fn encode_game_part(
        &self,
        game: &GameContent,
        part: &GamePart,
        policy: &PolicySet,
    ) -> Result<EncodedFile, std::io::Error> {
        Ok(EncodedFile {
            name: self.file_name_for_game_part(game, part, policy),
            backing: self.backing_for_game_part(part),
        })
    }

    fn encode_playlist(
        &self,
        game: &GameContent,
        entries: &[String],
        policy: &PolicySet,
    ) -> Result<EncodedFile, std::io::Error> {
        Ok(EncodedFile {
            name: self.playlist_name_for(game, policy),
            backing: self.playlist_backing(entries),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, ContentId, DiscPart, GameContent, GamePart, NormalizedContent, Platform,
        RomPart, TextContent,
    };
    use crate::core::source::SourceRef;

    #[test]
    fn encodes_bytes_content() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let content = NormalizedContent::Bytes(BytesContent {
            id: ContentId::new("bios"),
            source: SourceRef::new("file:/roms/bios"),
            size: 512,
        });

        let encoded = encoder.encode(&content, &policy).unwrap();
        assert_eq!(encoded.name, "bios.bin");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 512,
                source: SourceRef::new("file:/roms/bios"),
            }
        );
    }

    #[test]
    fn encodes_single_rom_game_content() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let content = NormalizedContent::Game(GameContent {
            id: ContentId::new("smw"),
            source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            title: "Super Mario World".to_string(),
            platform: Platform::Snes,
            parts: vec![GamePart::Rom(RomPart {
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                size: 4096,
            })],
            consumed_sources: vec![],
        });

        let encoded = encoder.encode(&content, &policy).unwrap();
        assert_eq!(encoded.name, "Super Mario World.sfc");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 4096,
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            }
        );
    }

    #[test]
    fn encodes_single_disc_game_content() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let content = NormalizedContent::Game(GameContent {
            id: ContentId::new("mgs"),
            source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
            title: "Metal Gear Solid".to_string(),
            platform: Platform::Ps1,
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/mgs-disc1.bin")],
            })],
            consumed_sources: vec![SourceRef::new("cue:/roms/mgs-disc1.bin")],
        });

        let encoded = encoder.encode(&content, &policy).unwrap();
        assert_eq!(encoded.name, "Metal Gear Solid (Disc 1).cue");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 0,
                source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
            }
        );
    }

    #[test]
    fn encodes_disc_part() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let game = GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![],
            consumed_sources: vec![],
        };

        let part = GamePart::Disc(DiscPart {
            source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
            disc_number: 2,
            consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc2.bin")],
        });

        let encoded = encoder.encode_game_part(&game, &part, &policy).unwrap();
        assert_eq!(encoded.name, "Final Fantasy VII (Disc 2).cue");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 0,
                source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
            }
        );
    }

    #[test]
    fn encodes_playlist() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let game = GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![],
            consumed_sources: vec![],
        };

        let encoded = encoder
            .encode_playlist(
                &game,
                &[
                    "Final Fantasy VII (Disc 1).cue".to_string(),
                    "Final Fantasy VII (Disc 2).cue".to_string(),
                ],
                &policy,
            )
            .unwrap();

        assert_eq!(encoded.name, "Final Fantasy VII.m3u");
        assert_eq!(
            encoded.backing,
            EncodedBacking::Inline(
                b"Final Fantasy VII (Disc 1).cue\nFinal Fantasy VII (Disc 2).cue\n".to_vec()
            )
        );
    }

    #[test]
    fn encodes_text_content() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let content = NormalizedContent::Text(TextContent {
            id: ContentId::new("readme"),
            source: SourceRef::new("file:/roms/readme"),
            size: 128,
        });

        let encoded = encoder.encode(&content, &policy).unwrap();
        assert_eq!(encoded.name, "readme.txt");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 128,
                source: SourceRef::new("file:/roms/readme"),
            }
        );
    }

    #[test]
    fn preserves_relative_path_for_text_content() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let content = NormalizedContent::Text(TextContent {
            id: ContentId::new("mixed/notes"),
            source: SourceRef::new("mixed/notes.txt"),
            size: 10,
        });

        let encoded = encoder.encode(&content, &policy).unwrap();
        assert_eq!(encoded.name, "mixed/notes.txt");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 10,
                source: SourceRef::new("mixed/notes.txt"),
            }
        );
    }

    #[test]
    fn normalizes_text_extension_from_id_path() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();
        let content = NormalizedContent::Text(TextContent {
            id: ContentId::new("roms/snes/game"),
            source: SourceRef::new("roms/snes/game.nfo"),
            size: 19,
        });

        let encoded = encoder.encode(&content, &policy).unwrap();
        assert_eq!(encoded.name, "roms/snes/game.txt");
        assert_eq!(
            encoded.backing,
            EncodedBacking::SourceBacked {
                size: 19,
                source: SourceRef::new("roms/snes/game.nfo"),
            }
        );
    }

    #[test]
    fn derives_rom_file_name_from_zip_member_source() {
        let encoder = BasicEncoder::new();
        let policy = PolicySet::default();

        let game = GameContent {
            id: ContentId::new("sonic"),
            source: SourceRef::new("zip:/roms/megadrive.zip#Sonic the Hedgehog.bin"),
            title: "Sonic the Hedgehog".to_string(),
            platform: Platform::Megadrive,
            parts: vec![],
            consumed_sources: vec![],
        };

        let encoded = encoder
            .encode_game_part(
                &game,
                &GamePart::Rom(RomPart {
                    source: SourceRef::new("zip:/roms/megadrive.zip#Sonic the Hedgehog.bin"),
                    size: 2048,
                }),
                &policy,
            )
            .unwrap();

        assert_eq!(encoded.name, "Sonic the Hedgehog.bin");
    }
}
