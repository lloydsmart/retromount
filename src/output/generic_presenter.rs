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
        let mut root = VfsDirectory::new("");

        for entry in entries {
            match &entry.content {
                Content::Game(game) => {
                    self.insert_game_node(&mut root, game, &entry.encoded);
                }
                Content::Bytes(_) | Content::Text(_) => {
                    self.insert_file_path(
                        &mut root,
                        &entry.encoded.name,
                        VfsFile::source_backed(
                            file_name(&entry.encoded.name),
                            entry.encoded.size,
                            entry.content.source().clone(),
                        ),
                    );
                }
                Content::Rom(_) | Content::Disc(_) => {
                    unreachable!("GenericPresenter should only receive normalized playable content")
                }
            }
        }

        root.children().to_vec()
    }

    fn insert_game_node(&self, root: &mut VfsDirectory, game: &GameContent, encoded: &EncodedFile) {
        let base_path = format!("{}/{}", game.platform, game.title);

        if self.is_multi_disc_game(game) {
            self.insert_multi_disc_game(root, game, &base_path);
        } else {
            let file_path = format!("{}/{}", base_path, file_name(&encoded.name));
            self.insert_file_path(
                root,
                &file_path,
                VfsFile::source_backed(file_name(&file_path), encoded.size, game.source.clone()),
            );
        }
    }

    fn insert_multi_disc_game(&self, root: &mut VfsDirectory, game: &GameContent, base_path: &str) {
        let mut disc_parts: Vec<&DiscPart> = game
            .parts
            .iter()
            .filter_map(|part| match part {
                GamePart::Disc(disc) => Some(disc),
                _ => None,
            })
            .collect();

        disc_parts.sort_by(|left, right| left.disc_number.cmp(&right.disc_number));

        let disc_names: Vec<String> = disc_parts
            .iter()
            .map(|disc| format!("{} (Disc {}).cue", game.title, disc.disc_number))
            .collect();

        for disc in &disc_parts {
            let disc_path = format!(
                "{}/{} (Disc {}).cue",
                base_path, game.title, disc.disc_number
            );
            self.insert_file_path(
                root,
                &disc_path,
                VfsFile::source_backed(file_name(&disc_path), 0, disc.source.clone()),
            );
        }

        let playlist_path = format!("{}/{}.m3u", base_path, game.title);
        let playlist = disc_names.join("\n") + "\n";
        self.insert_file_path(
            root,
            &playlist_path,
            VfsFile::inline(file_name(&playlist_path), playlist.into_bytes()),
        );
    }

    fn insert_file_path(&self, root: &mut VfsDirectory, path: &str, file: VfsFile) {
        let normalized = normalize_path(path);
        let (parents, _) = split_parent_dirs(&normalized);
        let directory = ensure_directory(root, &parents);
        directory.add_child(VfsNode::File(file));
    }

    fn is_multi_disc_game(&self, game: &GameContent) -> bool {
        game.parts.len() > 1
            && game
                .parts
                .iter()
                .all(|part| matches!(part, GamePart::Disc(_)))
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

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}

fn split_parent_dirs(path: &str) -> (Vec<String>, String) {
    let normalized = normalize_path(path);

    match normalized.rsplit_once('/') {
        Some((parent, file_name)) => (
            parent
                .split('/')
                .filter(|segment| !segment.is_empty())
                .map(|segment| segment.to_string())
                .collect(),
            file_name.to_string(),
        ),
        None => (Vec::new(), normalized),
    }
}

fn file_name(path: &str) -> String {
    let normalized = normalize_path(path);

    match normalized.rsplit_once('/') {
        Some((_, file_name)) => file_name.to_string(),
        None => normalized,
    }
}

fn ensure_directory<'a>(root: &'a mut VfsDirectory, parents: &[String]) -> &'a mut VfsDirectory {
    let mut current = root;

    for segment in parents {
        let existing_index = current
            .children
            .iter()
            .position(|node| matches!(node, VfsNode::Directory(dir) if dir.name == *segment));

        let index = match existing_index {
            Some(index) => index,
            None => {
                current.add_child(VfsNode::Directory(VfsDirectory::new(segment)));
                current.children.len() - 1
            }
        };

        current = match current.children.get_mut(index) {
            Some(VfsNode::Directory(dir)) => dir,
            _ => unreachable!("directory entry should be a directory"),
        };
    }

    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, Content, ContentId, DiscPart, GameContent, GamePart, Platform, RomPart,
        TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::core::vfs::FileBacking;
    use crate::output::basic_encoder::BasicEncoder;

    #[test]
    fn presents_mixed_content_in_library_view() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Bytes(BytesContent {
                id: ContentId::new("bios"),
                source: SourceRef::new("bios"),
                size: 512,
            }),
            Content::Game(GameContent {
                id: ContentId::new("sonic"),
                source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                title: "Sonic the Hedgehog".to_string(),
                platform: Platform::Megadrive,
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
                platform: Platform::Ps1,
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

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["bios.bin", "manifest.txt", "megadrive", "ps1"]);
    }

    #[test]
    fn presents_single_rom_game_as_file_in_platform_title_directory() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![Content::Game(GameContent {
            id: ContentId::new("smw"),
            source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            title: "Super Mario World".to_string(),
            platform: Platform::Snes,
            parts: vec![GamePart::Rom(RomPart {
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                file_name: "Super Mario World.sfc".to_string(),
                size: 4096,
            })],
            consumed_sources: vec![],
        })];

        let root = presenter.present(&content);

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "snes");

        let snes = match &root.children()[0] {
            VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        let game = match &snes.children()[0] {
            VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        assert_eq!(game.name, "Super Mario World");
        assert_eq!(game.children()[0].name(), "Super Mario World.sfc");
    }

    #[test]
    fn presents_multi_disc_game_as_directory_with_playlist_in_library_view() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![Content::Game(GameContent {
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

        assert_eq!(root.children()[0].name(), "ps1");

        let ps1 = match &root.children()[0] {
            VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        let game_dir = match &ps1.children()[0] {
            VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        let child_names: Vec<&str> = game_dir.children().iter().map(|node| node.name()).collect();
        assert_eq!(
            child_names,
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ]
        );

        let playlist = match &game_dir.children()[2] {
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
    fn presents_multiple_games_under_platform_directories() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Game(GameContent {
                id: ContentId::new("crash-bandicoot"),
                source: SourceRef::new("file:/roms/Crash Bandicoot.bin"),
                title: "Crash Bandicoot".to_string(),
                platform: Platform::Ps1,
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
        ];

        let root = presenter.present(&content);

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "ps1");

        let ps1 = match &root.children()[0] {
            VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        let names: Vec<&str> = ps1.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["Crash Bandicoot", "Final Fantasy VII"]);
    }
}
