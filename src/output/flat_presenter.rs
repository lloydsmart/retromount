use crate::core::content::NormalizedContent;
use crate::core::vfs::{VfsDirectory, VfsNode};
use crate::output::basic_encoder::BasicEncoder;
use crate::output::name_allocator::allocate_file_name;
use crate::output::present::OutputPresenter;
use crate::output::presented::{build_presented_entries, PresentedEntry, PresentedGame};
use crate::policy::PolicySet;

pub struct FlatPresenter {
    encoder: BasicEncoder,
}

impl FlatPresenter {
    pub fn new() -> Self {
        Self {
            encoder: BasicEncoder::new(),
        }
    }

    fn build_root_children(&self, entries: &[PresentedEntry], policy: &PolicySet) -> Vec<VfsNode> {
        let mut root = VfsDirectory::new("");

        for entry in entries {
            match entry {
                PresentedEntry::Game(presented) => {
                    self.insert_game_files(&mut root, presented, policy);
                }
                PresentedEntry::File(encoded) => {
                    let mut file = encoded.to_vfs_file();

                    if let Some(name) = file.name.rsplit('/').next() {
                        file.name = name.to_string();
                    }

                    file.name = allocate_file_name(&root, &file.name, policy);
                    root.add_child(VfsNode::File(file));
                }
            }
        }

        root.children().to_vec()
    }

    fn insert_game_files(
        &self,
        root: &mut VfsDirectory,
        presented: &PresentedGame,
        policy: &PolicySet,
    ) {
        for encoded in &presented.files {
            let mut encoded = encoded.clone();
            encoded.name = allocate_file_name(root, &encoded.name, policy);
            root.add_child(VfsNode::File(encoded.to_vfs_file()));
        }
    }
}

impl Default for FlatPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputPresenter for FlatPresenter {
    fn present(&self, content: &[NormalizedContent], policy: &PolicySet) -> VfsDirectory {
        let presented_entries = build_presented_entries(content, &self.encoder, policy);
        let root_children = self.build_root_children(&presented_entries, policy);

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
    use crate::policy::{ConflictPolicy, FormattingPolicy, NamingPolicy, PolicySet};

    struct DefaultLikeNamingPolicy;

    impl NamingPolicy for DefaultLikeNamingPolicy {
        fn game_name(&self, game: &GameContent) -> String {
            game.title.clone()
        }

        fn part_name(&self, game: &GameContent, part: &GamePart) -> String {
            match part {
                GamePart::Rom(rom) => rom.source.file_name(),
                GamePart::Disc(disc) => format!("{} (Disc {}).cue", game.title, disc.disc_number),
            }
        }

        fn playlist_name(&self, game: &GameContent) -> String {
            format!("{}.m3u", game.title)
        }

        fn platform_name(&self, platform: &Platform) -> String {
            platform.to_string()
        }
    }

    struct PassthroughFormattingPolicy;

    impl FormattingPolicy for PassthroughFormattingPolicy {
        fn format_name(&self, raw: &str) -> String {
            raw.to_string()
        }
    }

    struct SuffixConflictPolicy;

    impl ConflictPolicy for SuffixConflictPolicy {
        fn resolve_name_conflict(&self, proposed: &str, existing: &[String]) -> String {
            if !existing.iter().any(|name| name == proposed) {
                return proposed.to_string();
            }

            let mut i = 1;
            loop {
                let candidate = format!("{proposed} ({i})");
                if !existing.iter().any(|name| name == &candidate) {
                    return candidate;
                }
                i += 1;
            }
        }
    }

    fn suffix_conflict_policy() -> PolicySet {
        PolicySet::new(
            Box::new(DefaultLikeNamingPolicy),
            Box::new(PassthroughFormattingPolicy),
            Box::new(SuffixConflictPolicy),
        )
    }

    #[test]
    fn presents_mixed_content_at_root() {
        let presenter = FlatPresenter::new();
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

        let root = presenter.present(&content, &policy);

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

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["Super Mario World.sfc"]);
    }

    #[test]
    fn presents_multi_disc_game_as_root_files_with_playlist() {
        let presenter = FlatPresenter::new();
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
            crate::core::vfs::FileBacking::Inline(contents) => {
                assert_eq!(
                    contents,
                    b"Final Fantasy VII (Disc 1).cue\nFinal Fantasy VII (Disc 2).cue\n"
                );
            }
            other => panic!("expected inline backing, got {other:?}"),
        }
    }

    #[test]
    fn flattens_non_game_content_to_leaf_file_names() {
        let presenter = FlatPresenter::new();
        let policy = PolicySet::default();

        let content = vec![
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("mixed/nested.zip"),
                source: SourceRef::new("file:/roms/mixed/nested.zip"),
                size: 197,
            }),
            NormalizedContent::Text(TextContent {
                id: ContentId::new("mixed/notes"),
                source: SourceRef::new("file:/roms/mixed/notes.txt"),
                size: 12,
            }),
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("roms/snes/cover.jpg"),
                source: SourceRef::new("file:/roms/roms/snes/cover.jpg"),
                size: 12,
            }),
        ];

        let root = presenter.present(&content, &policy);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["cover.jpg.bin", "nested.zip.bin", "notes.txt"]);
    }

    #[test]
    fn conflict_policy_can_disambiguate_flat_sibling_file_names() {
        let presenter = FlatPresenter::new();
        let policy = suffix_conflict_policy();

        let content = vec![
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("docs/readme"),
                source: SourceRef::new("file:/roms/docs/readme"),
                size: 10,
            }),
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("other/readme"),
                source: SourceRef::new("file:/roms/other/readme"),
                size: 11,
            }),
        ];

        let root = presenter.present(&content, &policy);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["readme.bin", "readme.bin (1)"]);
    }
}
