use crate::core::content::{DiscPart, GameContent, GamePart, NormalizedContent};
use crate::core::vfs::{VfsDirectory, VfsNode};
use crate::output::basic_encoder::BasicEncoder;
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::output::present::OutputPresenter;

pub struct FlatPresenter {
    encoder: BasicEncoder,
}

#[derive(Debug, Clone)]
struct EncodedEntry {
    content: NormalizedContent,
    encoded: EncodedFile,
}

impl FlatPresenter {
    pub fn new() -> Self {
        Self {
            encoder: BasicEncoder::new(),
        }
    }

    fn encode_entries(&self, content: &[NormalizedContent]) -> Vec<EncodedEntry> {
        content
            .iter()
            .filter(|item| self.encoder.can_encode(item))
            .filter_map(|item| {
                self.encoder.encode(item).ok().map(|encoded| EncodedEntry {
                    content: item.clone(),
                    encoded,
                })
            })
            .collect()
    }

    fn build_root_children(&self, entries: &[EncodedEntry]) -> Vec<VfsNode> {
        let mut root = VfsDirectory::new("");

        for entry in entries {
            match &entry.content {
                NormalizedContent::Game(game) => {
                    self.insert_game_node(&mut root, game, &entry.encoded);
                }
                NormalizedContent::Bytes(_) | NormalizedContent::Text(_) => {
                    root.add_child(VfsNode::File(entry.encoded.to_vfs_file()));
                }
            }
        }

        root.children().to_vec()
    }

    fn insert_game_node(&self, root: &mut VfsDirectory, game: &GameContent, encoded: &EncodedFile) {
        if self.is_multi_disc_game(game) {
            self.insert_multi_disc_game(root, game);
        } else {
            root.add_child(VfsNode::File(encoded.to_vfs_file()));
        }
    }

    fn insert_multi_disc_game(&self, root: &mut VfsDirectory, game: &GameContent) {
        let mut disc_parts: Vec<&DiscPart> = game
            .parts
            .iter()
            .filter_map(|part| match part {
                GamePart::Disc(disc) => Some(disc),
                _ => None,
            })
            .collect();

        disc_parts.sort_by(|left, right| left.disc_number.cmp(&right.disc_number));

        let mut disc_names = Vec::new();

        for disc in &disc_parts {
            let encoded = self
                .encoder
                .encode_game_part(game, &GamePart::Disc((*disc).clone()))
                .unwrap_or_else(|err| {
                    panic!(
                        "multi-disc game part should encode for '{}' disc {}: {err}",
                        game.title, disc.disc_number
                    )
                });

            disc_names.push(encoded.name.clone());
            root.add_child(VfsNode::File(encoded.to_vfs_file()));
        }

        let playlist_encoded = self
            .encoder
            .encode_playlist(game, &disc_names)
            .unwrap_or_else(|err| {
                panic!(
                    "multi-disc playlist should encode for '{}': {err}",
                    game.title
                )
            });

        root.add_child(VfsNode::File(playlist_encoded.to_vfs_file()));
    }

    fn is_multi_disc_game(&self, game: &GameContent) -> bool {
        game.parts.len() > 1
            && game
                .parts
                .iter()
                .all(|part| matches!(part, GamePart::Disc(_)))
    }
}

impl Default for FlatPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputPresenter for FlatPresenter {
    fn present(&self, content: &[NormalizedContent]) -> VfsDirectory {
        let encoded_entries = self.encode_entries(content);
        let root_children = self.build_root_children(&encoded_entries);

        VfsDirectory::with_children("", root_children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, ContentId, DiscPart, GameContent, GamePart, Platform, RomPart, TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::core::vfs::FileBacking;

    #[test]
    fn presents_mixed_content_at_root() {
        let presenter = FlatPresenter::new();

        let content = vec![
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("bios"),
                source: SourceRef::new("bios"),
                size: 512,
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("sonic"),
                source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                title: "Sonic the Hedgehog".to_string(),
                platform: Platform::Megadrive,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                    size: 1024,
                })],
                consumed_sources: vec![],
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("ff7"),
                source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
                title: "Final Fantasy VII".to_string(),
                platform: Platform::Ps1,
                parts: vec![
                    GamePart::Disc(DiscPart {
                        source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
                        disc_number: 2,
                        consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc2.bin")],
                    }),
                    GamePart::Disc(DiscPart {
                        source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
                        disc_number: 1,
                        consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc1.bin")],
                    }),
                ],
                consumed_sources: vec![
                    SourceRef::new("cue:/roms/ff7-disc2.bin"),
                    SourceRef::new("cue:/roms/ff7-disc1.bin"),
                ],
            }),
            NormalizedContent::Text(TextContent {
                id: ContentId::new("manifest"),
                source: SourceRef::new("file:/roms/manifest"),
                size: 64,
            }),
        ];

        let root = presenter.present(&content);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(
            names,
            vec![
                "bios.bin",
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
                "manifest.txt",
                "sonic.bin",
            ]
        );
    }

    #[test]
    fn presents_single_rom_game_as_root_file() {
        let presenter = FlatPresenter::new();

        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("smw"),
            source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            title: "Super Mario World".to_string(),
            platform: Platform::Snes,
            parts: vec![GamePart::Rom(RomPart {
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                size: 4096,
            })],
            consumed_sources: vec![],
        })];

        let root = presenter.present(&content);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["Super Mario World.sfc"]);
    }

    #[test]
    fn presents_multi_disc_game_as_root_files_with_playlist() {
        let presenter = FlatPresenter::new();

        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
                    disc_number: 2,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc2.bin")],
                }),
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc1.bin")],
                }),
            ],
            consumed_sources: vec![
                SourceRef::new("cue:/roms/ff7-disc2.bin"),
                SourceRef::new("cue:/roms/ff7-disc1.bin"),
            ],
        })];

        let root = presenter.present(&content);

        let child_names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(
            child_names,
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ]
        );

        let playlist = match &root.children()[2] {
            VfsNode::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &playlist.backing {
            FileBacking::Inline(contents) => {
                assert_eq!(
                    contents,
                    b"Final Fantasy VII (Disc 1).cue\nFinal Fantasy VII (Disc 2).cue\n"
                );
            }
            other => panic!("expected inline backing, got {other:?}"),
        }
    }
}
