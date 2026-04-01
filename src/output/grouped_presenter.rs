use crate::core::content::{DiscPart, GameContent, GamePart, NormalizedContent};
use crate::core::vfs::{VfsDirectory, VfsNode};
use crate::output::basic_encoder::BasicEncoder;
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::output::present::OutputPresenter;
use crate::policy::PolicySet;

pub struct GroupedPresenter {
    encoder: BasicEncoder,
}

#[derive(Debug, Clone)]
struct EncodedEntry {
    content: NormalizedContent,
    encoded: EncodedFile,
}

impl GroupedPresenter {
    pub fn new() -> Self {
        Self {
            encoder: BasicEncoder::new(),
        }
    }

    fn encode_entries(
        &self,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> Vec<EncodedEntry> {
        content
            .iter()
            .filter(|item| self.encoder.can_encode(item))
            .filter_map(|item| {
                self.encoder
                    .encode(item, policy)
                    .ok()
                    .map(|encoded| EncodedEntry {
                        content: item.clone(),
                        encoded,
                    })
            })
            .collect()
    }

    fn build_root_children(&self, entries: &[EncodedEntry], policy: &PolicySet) -> Vec<VfsNode> {
        let mut root = VfsDirectory::new("");

        for entry in entries {
            match &entry.content {
                NormalizedContent::Game(game) => {
                    self.insert_game_node(&mut root, game, &entry.encoded, policy);
                }
                NormalizedContent::Bytes(_) | NormalizedContent::Text(_) => {
                    self.insert_file_path(
                        &mut root,
                        &entry.encoded.name,
                        entry.encoded.to_vfs_file(),
                    );
                }
            }
        }

        root.children().to_vec()
    }

    fn insert_game_node(
        &self,
        root: &mut VfsDirectory,
        game: &GameContent,
        encoded: &EncodedFile,
        policy: &PolicySet,
    ) {
        let platform_name = policy.naming().platform_name(&game.platform);
        let game_name = policy.naming().game_name(game);

        let base_path = format!(
            "{}/{}",
            policy.format_name(&platform_name),
            policy.format_name(&game_name)
        );

        if self.is_multi_disc_game(game) {
            self.insert_multi_disc_game(root, game, &base_path, policy);
        } else {
            let file_path = format!("{}/{}", base_path, encoded.name);
            self.insert_file_path(root, &file_path, encoded.to_vfs_file());
        }
    }

    fn insert_multi_disc_game(
        &self,
        root: &mut VfsDirectory,
        game: &GameContent,
        base_path: &str,
        policy: &PolicySet,
    ) {
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
                .encode_game_part(game, &GamePart::Disc((*disc).clone()), policy)
                .unwrap_or_else(|err| {
                    panic!(
                        "multi-disc game part should encode for '{}' disc {}: {err}",
                        game.title, disc.disc_number
                    )
                });

            disc_names.push(encoded.name.clone());

            let disc_path = format!("{}/{}", base_path, encoded.name);
            self.insert_file_path(root, &disc_path, encoded.to_vfs_file());
        }

        let playlist_encoded = self
            .encoder
            .encode_playlist(game, &disc_names, policy)
            .unwrap_or_else(|err| {
                panic!(
                    "multi-disc playlist should encode for '{}': {err}",
                    game.title
                )
            });

        let playlist_path = format!("{}/{}", base_path, playlist_encoded.name);
        self.insert_file_path(root, &playlist_path, playlist_encoded.to_vfs_file());
    }

    fn insert_file_path(
        &self,
        root: &mut VfsDirectory,
        path: &str,
        file: crate::core::vfs::VfsFile,
    ) {
        let normalized = normalize_path(path);
        let (parents, file_name) = split_parent_dirs(&normalized);
        let directory = ensure_directory(root, &parents);

        let mut file = file;
        file.name = file_name;

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

impl Default for GroupedPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputPresenter for GroupedPresenter {
    fn present(&self, content: &[NormalizedContent], policy: &PolicySet) -> VfsDirectory {
        let encoded_entries = self.encode_entries(content, policy);
        let root_children = self.build_root_children(&encoded_entries, policy);

        VfsDirectory::with_children("", root_children)
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

fn ensure_directory<'a>(root: &'a mut VfsDirectory, parents: &[String]) -> &'a mut VfsDirectory {
    let mut current = root;

    for segment in parents {
        let index = match current
            .children
            .iter()
            .position(|node| matches!(node, VfsNode::Directory(dir) if dir.name == *segment))
        {
            Some(index) => index,
            None => {
                current.add_child(VfsNode::Directory(VfsDirectory::new(segment)));

                current
                    .children
                    .iter()
                    .position(
                        |node| matches!(node, VfsNode::Directory(dir) if dir.name == *segment),
                    )
                    .expect("inserted directory should be present")
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
        BytesContent, ContentId, DiscPart, GameContent, GamePart, Platform, RomPart, TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::core::vfs::FileBacking;
    use crate::policy::{ConflictPolicy, FormattingPolicy, NamingPolicy, PolicySet};

    struct AlternateNamingPolicy;

    impl NamingPolicy for AlternateNamingPolicy {
        fn game_name(&self, game: &GameContent) -> String {
            format!("{} [ALT]", game.title)
        }

        fn part_name(&self, game: &GameContent, part: &GamePart) -> String {
            match part {
                GamePart::Rom(rom) => format!("ALT-{}", rom.source.file_name()),
                GamePart::Disc(disc) => format!("{} - CD{}.cue", game.title, disc.disc_number),
            }
        }

        fn playlist_name(&self, game: &GameContent) -> String {
            format!("{} - Playlist.m3u", game.title)
        }

        fn platform_name(&self, platform: &Platform) -> String {
            match platform {
                Platform::Ps1 => "PlayStation".to_string(),
                _ => platform.to_string(),
            }
        }
    }

    struct PassthroughFormattingPolicy;

    impl FormattingPolicy for PassthroughFormattingPolicy {
        fn format_name(&self, raw: &str) -> String {
            raw.to_string()
        }
    }

    struct PreserveConflictPolicy;

    impl ConflictPolicy for PreserveConflictPolicy {
        fn resolve_name_conflict(&self, proposed: &str, _existing: &[String]) -> String {
            proposed.to_string()
        }
    }

    fn alternate_policy() -> PolicySet {
        PolicySet::new(
            Box::new(AlternateNamingPolicy),
            Box::new(PassthroughFormattingPolicy),
            Box::new(PreserveConflictPolicy),
        )
    }

    #[test]
    fn presents_mixed_content_in_library_view() {
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();

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
            NormalizedContent::Text(TextContent {
                id: ContentId::new("manifest"),
                source: SourceRef::new("file:/roms/manifest"),
                size: 64,
            }),
        ];

        let root = presenter.present(&content, &policy);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["megadrive", "ps1", "bios.bin", "manifest.txt"]);
    }

    #[test]
    fn presents_single_rom_game_as_file_in_platform_title_directory() {
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();

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

        let root = presenter.present(&content, &policy);

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
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();

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

        let root = presenter.present(&content, &policy);

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
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();

        let content = vec![
            NormalizedContent::Game(GameContent {
                id: ContentId::new("crash-bandicoot"),
                source: SourceRef::new("file:/roms/Crash Bandicoot.bin"),
                title: "Crash Bandicoot".to_string(),
                platform: Platform::Ps1,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/Crash Bandicoot.bin"),
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
        ];

        let root = presenter.present(&content, &policy);

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "ps1");

        let ps1 = match &root.children()[0] {
            VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        let names: Vec<&str> = ps1.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["Crash Bandicoot", "Final Fantasy VII"]);
    }

    #[test]
    fn presents_text_content_with_relative_path_using_leaf_file_name() {
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();

        let content = vec![NormalizedContent::Text(TextContent {
            id: ContentId::new("mixed/notes"),
            source: SourceRef::new("file:/roms/mixed/notes.txt"),
            size: 12,
        })];

        let root = presenter.present(&content, &policy);

        let mixed = match root.find_directory("mixed") {
            Some(dir) => dir,
            None => panic!("expected mixed directory"),
        };

        let names: Vec<&str> = mixed.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["notes.txt"]);
    }

    #[test]
    fn presents_bytes_content_with_relative_path_using_leaf_file_name() {
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();

        let content = vec![NormalizedContent::Bytes(BytesContent {
            id: ContentId::new("zips/gameboy_collection.zip"),
            source: SourceRef::new("file:/roms/zips/gameboy_collection.zip"),
            size: 481,
        })];

        let root = presenter.present(&content, &policy);

        let zips = match root.find_directory("zips") {
            Some(dir) => dir,
            None => panic!("expected zips directory"),
        };

        let names: Vec<&str> = zips.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["gameboy_collection.zip.bin"]);
    }

    #[test]
    fn naming_policy_can_change_grouped_output_names_without_changing_structure() {
        let presenter = GroupedPresenter::new();
        let default_policy = PolicySet::default();
        let alt_policy = alternate_policy();

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

        let default_root = presenter.present(&content, &default_policy);
        let alt_root = presenter.present(&content, &alt_policy);

        let default_platform = default_root.find_directory("ps1").unwrap();
        let default_game = default_platform
            .find_directory("Final Fantasy VII")
            .unwrap();
        let default_names: Vec<&str> = default_game
            .children()
            .iter()
            .map(|node| node.name())
            .collect();
        assert_eq!(
            default_names,
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ]
        );

        let alt_platform = alt_root.find_directory("PlayStation").unwrap();
        let alt_game = alt_platform
            .find_directory("Final Fantasy VII [ALT]")
            .unwrap();
        let alt_names: Vec<&str> = alt_game.children().iter().map(|node| node.name()).collect();
        assert_eq!(
            alt_names,
            vec![
                "Final Fantasy VII - CD1.cue",
                "Final Fantasy VII - CD2.cue",
                "Final Fantasy VII - Playlist.m3u",
            ]
        );

        assert_eq!(default_root.children().len(), 1);
        assert_eq!(alt_root.children().len(), 1);
    }
}
