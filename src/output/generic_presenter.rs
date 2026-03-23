use crate::core::content::{Content, DiscPart, GameContent, GamePart};
use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::output::present::OutputPresenter;

pub struct GenericPresenter<E>
where
    E: OutputEncoder,
{
    encoder: E,
}

#[derive(Debug, Clone)]
struct PresentedEntry {
    content: Content,
    encoded: EncodedFile,
}

impl<E> GenericPresenter<E>
where
    E: OutputEncoder,
{
    pub fn new(encoder: E) -> Self {
        Self { encoder }
    }

    fn encode_entries(&self, content: &[Content]) -> Vec<PresentedEntry> {
        content
            .iter()
            .filter(|item| self.encoder.can_encode(item))
            .filter_map(|item| {
                self.encoder
                    .encode(item)
                    .ok()
                    .map(|encoded| PresentedEntry {
                        content: item.clone(),
                        encoded,
                    })
            })
            .collect()
    }

    fn build_root_children(&self, entries: &[PresentedEntry]) -> Vec<VfsNode> {
        let mut children = Vec::new();

        for entry in entries {
            match &entry.content {
                Content::Game(game) => {
                    children.push(self.build_game_node(game, &entry.encoded));
                }
                Content::Bytes(_) | Content::Text(_) => {
                    children.push(VfsNode::File(VfsFile::source_backed(
                        entry.encoded.name.clone(),
                        entry.encoded.size,
                        entry.content.source().clone(),
                    )));
                }
                Content::Rom(_) | Content::Disc(_) => {
                    unreachable!("GenericPresenter should only receive normalized playable content")
                }
            }
        }

        children
    }

    fn build_game_node(&self, game: &GameContent, encoded: &EncodedFile) -> VfsNode {
        if self.is_multi_disc_game(game) {
            VfsNode::Directory(self.build_multi_disc_game_directory(game))
        } else {
            VfsNode::File(VfsFile::source_backed(
                encoded.name.clone(),
                encoded.size,
                game.source.clone(),
            ))
        }
    }

    fn is_multi_disc_game(&self, game: &GameContent) -> bool {
        game.parts.len() > 1
            && game
                .parts
                .iter()
                .all(|part| matches!(part, GamePart::Disc(_)))
    }

    fn build_multi_disc_game_directory(&self, game: &GameContent) -> VfsDirectory {
        let mut disc_parts: Vec<&DiscPart> = game
            .parts
            .iter()
            .filter_map(|part| match part {
                GamePart::Disc(disc) => Some(disc),
                _ => None,
            })
            .collect();

        disc_parts.sort_by(|left, right| left.disc_number.cmp(&right.disc_number));

        let disc_file_names: Vec<String> = disc_parts
            .iter()
            .map(|disc| format!("{} (Disc {}).cue", game.title, disc.disc_number))
            .collect();

        let mut dir = VfsDirectory::new(&game.title);

        for disc in disc_parts {
            dir.add_child(VfsNode::File(VfsFile::source_backed(
                format!("{} (Disc {}).cue", game.title, disc.disc_number),
                0,
                disc.source.clone(),
            )));
        }

        dir.add_child(VfsNode::File(
            self.build_m3u_file(&game.title, &disc_file_names),
        ));

        dir
    }

    fn build_m3u_file(&self, title: &str, disc_file_names: &[String]) -> VfsFile {
        let playlist = disc_file_names.join("\n") + "\n";
        VfsFile::inline(format!("{title}.m3u"), playlist.into_bytes())
    }
}

impl<E> OutputPresenter for GenericPresenter<E>
where
    E: OutputEncoder + Send + Sync,
{
    fn present(&self, content: &[Content]) -> VfsDirectory {
        let entries = self.encode_entries(content);
        let children = self.build_root_children(&entries);

        VfsDirectory::with_children("", children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, Content, ContentId, DiscPart, GameContent, GamePart, RomPart, TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::core::vfs::FileBacking;
    use crate::output::basic_encoder::BasicEncoder;

    #[test]
    fn presents_mixed_content_as_root_files() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Bytes(BytesContent {
                id: ContentId::new("bios"),
                source: SourceRef::new("file:/roms/bios"),
                size: 512,
            }),
            Content::Game(GameContent {
                id: ContentId::new("sonic"),
                source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                title: "Sonic the Hedgehog".to_string(),
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                    file_name: "Sonic the Hedgehog.bin".to_string(),
                    size: 1024,
                })],
                consumed_sources: vec![],
            }),
            Content::Game(GameContent {
                id: ContentId::new("ff7"),
                source: SourceRef::new("cue:/roms/ff7.cue"),
                title: "Final Fantasy VII".to_string(),
                parts: vec![GamePart::Disc(DiscPart {
                    source: SourceRef::new("cue:/roms/ff7.cue"),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ff7.bin")],
                })],
                consumed_sources: vec![SourceRef::new("cue:/roms/ff7.bin")],
            }),
            Content::Text(TextContent {
                id: ContentId::new("manifest"),
                source: SourceRef::new("file:/roms/manifest"),
                size: 64,
            }),
        ];

        let root = presenter.present(&content);

        assert_eq!(root.name, "");
        assert_eq!(root.children().len(), 4);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(
            names,
            vec![
                "bios.bin",
                "Final Fantasy VII (Disc 1).cue",
                "manifest.txt",
                "Sonic the Hedgehog.bin",
            ]
        );

        match &root.children()[0] {
            VfsNode::File(file) => match &file.backing {
                FileBacking::Source(source) => {
                    assert_eq!(source.to_string(), "file:/roms/bios");
                }
                other => panic!("expected source backing, got {other:?}"),
            },
            other => panic!("expected file, got {other:?}"),
        }
    }

    #[test]
    fn presents_single_rom_game_as_file() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![Content::Game(GameContent {
            id: ContentId::new("smw"),
            source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            title: "Super Mario World".to_string(),
            parts: vec![GamePart::Rom(RomPart {
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                file_name: "Super Mario World.sfc".to_string(),
                size: 4096,
            })],
            consumed_sources: vec![],
        })];

        let root = presenter.present(&content);

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "Super Mario World.sfc");
    }

    #[test]
    fn presents_multi_disc_game_as_directory_with_playlist() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![Content::Game(GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
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

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "Final Fantasy VII");

        let directory = match &root.children()[0] {
            VfsNode::Directory(directory) => directory,
            other => panic!("expected directory, got {other:?}"),
        };

        let child_names: Vec<&str> = directory
            .children()
            .iter()
            .map(|node| node.name())
            .collect();
        assert_eq!(
            child_names,
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ]
        );

        let playlist = match &directory.children()[2] {
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

    #[test]
    fn groups_multi_disc_sets_and_generates_playlist() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Game(GameContent {
                id: ContentId::new("crash-bandicoot"),
                source: SourceRef::new("file:/roms/Crash Bandicoot.bin"),
                title: "Crash Bandicoot".to_string(),
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/Crash Bandicoot.bin"),
                    file_name: "Crash Bandicoot.bin".to_string(),
                    size: 1024,
                })],
                consumed_sources: vec![],
            }),
            Content::Game(GameContent {
                id: ContentId::new("ff7"),
                source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
                title: "Final Fantasy VII".to_string(),
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
        ];

        let root = presenter.present(&content);

        assert_eq!(root.children().len(), 2);
        assert_eq!(root.children()[0].name(), "Final Fantasy VII");
        assert_eq!(root.children()[1].name(), "Crash Bandicoot.bin");
    }
}
