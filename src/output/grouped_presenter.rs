use crate::core::content::{
    BytesContent, DiscPart, GameContent, GamePart, NormalizedContent, TextContent,
};
use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
use crate::output::encode::OutputEncoder;
use crate::output::plan::{
    ArtifactId, ArtifactReference, ArtifactRequest, GeneratedArtifact, PlanDirectory, PlanEntry,
    PlanFile, PlannedArtifactKind, PlaylistArtifact, PresentationPlan, SourceArtifact,
};
use crate::output::present::OutputPresenter;
use crate::output::presentation_expansion::{is_multi_disc_game, sorted_disc_parts};
use crate::policy::PolicySet;

pub struct GroupedPresenter;

impl GroupedPresenter {
    pub fn new(_encoder: Box<dyn OutputEncoder>) -> Self {
        Self
    }

    fn build_root_entries(
        &self,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> Vec<PlanEntry> {
        let mut root = PlanDirectory::new("", Vec::new());

        for item in content {
            match item {
                NormalizedContent::Game(game) => self.insert_game_node(&mut root, game, policy),
                NormalizedContent::Bytes(bytes) => self.insert_bytes_path(&mut root, bytes, policy),
                NormalizedContent::Text(text) => self.insert_text_path(&mut root, text, policy),
            }
        }

        root.entries
    }

    fn insert_game_node(&self, root: &mut PlanDirectory, game: &GameContent, policy: &PolicySet) {
        let platform_name = policy.format_name(&policy.naming().platform_name(&game.platform));
        let game_name = policy.format_name(&policy.naming().game_name(game));

        let platform_dir = ensure_directory(root, &[platform_name], policy);
        let game_dir = ensure_distinct_child_directory(platform_dir, &game_name, policy);

        if is_multi_disc_game(game) {
            self.insert_multi_disc_game_files(game_dir, game, policy);
        } else {
            self.insert_single_game_file(game_dir, game, policy);
        }
    }

    fn insert_single_game_file(
        &self,
        game_dir: &mut PlanDirectory,
        game: &GameContent,
        policy: &PolicySet,
    ) {
        let proposed_name = match game.parts.first() {
            Some(part) => policy.naming().part_name(game, part),
            None => policy.naming().game_name(game),
        };

        let resolved_name = resolve_file_name(game_dir, &proposed_name, policy);

        let artifact = match game.parts.first() {
            Some(GamePart::Rom(rom)) => ArtifactRequest::new(
                ArtifactId::new(format!("game:{}", game.id)),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    rom.source.clone(),
                    rom.size,
                )),
                CapabilityRequirements::new(ContentType::Rom).with_format(Format::Bin),
            ),
            Some(GamePart::Disc(disc)) => ArtifactRequest::new(
                ArtifactId::new(format!("game:{}", game.id)),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    disc.source.clone(),
                    estimate_disc_size(game, disc),
                )),
                CapabilityRequirements::new(ContentType::Disc).with_format(Format::Bin),
            ),
            None => ArtifactRequest::new(
                ArtifactId::new(format!("game:{}", game.id)),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(game.source.clone(), 0)),
                CapabilityRequirements::new(ContentType::Game),
            ),
        };

        game_dir
            .entries
            .push(PlanEntry::File(PlanFile::new(resolved_name, artifact)));
    }

    fn insert_multi_disc_game_files(
        &self,
        game_dir: &mut PlanDirectory,
        game: &GameContent,
        policy: &PolicySet,
    ) {
        let mut disc_refs = Vec::new();

        for disc in sorted_disc_parts(game) {
            let proposed_name = policy
                .naming()
                .part_name(game, &GamePart::Disc(disc.clone()));
            let resolved_name = resolve_file_name(game_dir, &proposed_name, policy);

            let artifact_id =
                ArtifactId::new(format!("game:{}:disc:{}", game.id, disc.disc_number));
            disc_refs.push(ArtifactReference::new(artifact_id.clone()));

            game_dir.entries.push(PlanEntry::File(PlanFile::new(
                resolved_name,
                ArtifactRequest::new(
                    artifact_id,
                    PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                        disc.source.clone(),
                        estimate_disc_size(game, disc),
                    )),
                    CapabilityRequirements::new(ContentType::Disc).with_format(Format::Bin),
                ),
            )));
        }

        let playlist_name =
            resolve_file_name(game_dir, &policy.naming().playlist_name(game), policy);

        game_dir.entries.push(PlanEntry::File(PlanFile::new(
            playlist_name,
            ArtifactRequest::new(
                ArtifactId::new(format!("game:{}:playlist", game.id)),
                PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(PlaylistArtifact::new(
                    disc_refs,
                ))),
                CapabilityRequirements::new(ContentType::Playlist).with_format(Format::M3u),
            ),
        )));
    }

    fn insert_bytes_path(
        &self,
        root: &mut PlanDirectory,
        bytes: &BytesContent,
        policy: &PolicySet,
    ) {
        let (parents, _) = split_parent_dirs(&bytes.id.to_string());
        let directory = ensure_directory(root, &parents, policy);
        let resolved_name = resolve_file_name(
            directory,
            &format!("{}.bin", bytes.source.file_name()),
            policy,
        );

        directory.entries.push(PlanEntry::File(PlanFile::new(
            resolved_name,
            ArtifactRequest::new(
                ArtifactId::new(format!("bytes:{}", bytes.id)),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    bytes.source.clone(),
                    bytes.size,
                )),
                CapabilityRequirements::new(ContentType::Bytes).with_format(Format::Bin),
            ),
        )));
    }

    fn insert_text_path(&self, root: &mut PlanDirectory, text: &TextContent, policy: &PolicySet) {
        let (parents, _) = split_parent_dirs(&text.id.to_string());
        let directory = ensure_directory(root, &parents, policy);
        let resolved_name = resolve_file_name(directory, &text_file_name(text), policy);

        directory.entries.push(PlanEntry::File(PlanFile::new(
            resolved_name,
            ArtifactRequest::new(
                ArtifactId::new(format!("text:{}", text.id)),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    text.source.clone(),
                    text.size,
                )),
                CapabilityRequirements::new(ContentType::Text).with_format(Format::Text),
            ),
        )));
    }
}

impl OutputPresenter for GroupedPresenter {
    fn present(&self, content: &[NormalizedContent], policy: &PolicySet) -> PresentationPlan {
        PresentationPlan::new(self.build_root_entries(content, policy))
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

fn ensure_directory<'a>(
    root: &'a mut PlanDirectory,
    parents: &[String],
    policy: &PolicySet,
) -> &'a mut PlanDirectory {
    let mut current = root;

    for segment in parents {
        let existing_index = current
            .entries
            .iter()
            .position(|entry| matches!(entry, PlanEntry::Directory(dir) if dir.name == *segment));

        let index = match existing_index {
            Some(index) => index,
            None => {
                let existing_names = child_names(current);
                let resolved_name = policy.resolve_name_conflict(segment, &existing_names);

                current
                    .entries
                    .push(PlanEntry::Directory(PlanDirectory::new(
                        &resolved_name,
                        Vec::new(),
                    )));

                current
                    .entries
                    .iter()
                    .position(|entry| {
                        matches!(entry, PlanEntry::Directory(dir) if dir.name == resolved_name)
                    })
                    .expect("inserted directory should be present")
            }
        };

        current = match current.entries.get_mut(index) {
            Some(PlanEntry::Directory(dir)) => dir,
            _ => unreachable!("directory entry should be a directory"),
        };
    }

    current
}

fn ensure_distinct_child_directory<'a>(
    parent: &'a mut PlanDirectory,
    proposed: &str,
    policy: &PolicySet,
) -> &'a mut PlanDirectory {
    let existing_names = child_names(parent);
    let resolved_name = policy.resolve_name_conflict(proposed, &existing_names);

    parent.entries.push(PlanEntry::Directory(PlanDirectory::new(
        &resolved_name,
        Vec::new(),
    )));

    let index = parent
        .entries
        .iter()
        .position(|entry| matches!(entry, PlanEntry::Directory(dir) if dir.name == resolved_name))
        .expect("inserted child directory should be present");

    match parent.entries.get_mut(index) {
        Some(PlanEntry::Directory(dir)) => dir,
        _ => unreachable!("child directory entry should be a directory"),
    }
}

fn resolve_file_name(parent: &PlanDirectory, proposed: &str, policy: &PolicySet) -> String {
    let existing_names = child_names(parent);
    policy.resolve_name_conflict(proposed, &existing_names)
}

fn child_names(directory: &PlanDirectory) -> Vec<String> {
    directory
        .entries
        .iter()
        .map(|entry| match entry {
            PlanEntry::Directory(dir) => dir.name.clone(),
            PlanEntry::File(file) => file.name.clone(),
        })
        .collect()
}

fn estimate_disc_size(_game: &GameContent, _disc: &DiscPart) -> u64 {
    // TODO(phase5b): carry authoritative source-backed sizes for disc artifacts
    // through the presentation plan. Disc cues currently materialize with size 0.
    0
}

fn text_file_name(text: &TextContent) -> String {
    let file_name = text.source.file_name();

    if file_name.contains('.') {
        file_name
    } else {
        format!("{file_name}.txt")
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
    use crate::output::materialize::materialize_presentation_plan;
    use crate::output::plan::{PlanEntry, PresentationPlan};
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

    fn alternate_policy() -> PolicySet {
        PolicySet::new(
            Box::new(AlternateNamingPolicy),
            Box::new(PassthroughFormattingPolicy),
            Box::new(PreserveConflictPolicy),
        )
    }

    fn suffix_conflict_policy() -> PolicySet {
        PolicySet::new(
            Box::new(DefaultLikeNamingPolicy),
            Box::new(PassthroughFormattingPolicy),
            Box::new(SuffixConflictPolicy),
        )
    }

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

    fn render_root(
        presenter: &GroupedPresenter,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> crate::core::vfs::VfsDirectory {
        let plan = presenter.present(content, policy);
        materialize_presentation_plan(&plan).unwrap()
    }

    fn render_plan(
        presenter: &GroupedPresenter,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> PresentationPlan {
        presenter.present(content, policy)
    }

    #[test]
    fn grouped_presenter_plan_places_game_under_platform_directory() {
        let presenter = GroupedPresenter;
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

        let plan = render_plan(&presenter, &content, &policy);

        assert_eq!(plan.entries.len(), 1);

        let platform_dir = match &plan.entries[0] {
            PlanEntry::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        assert_eq!(platform_dir.name, "snes");

        let game_dir = match &platform_dir.entries[0] {
            PlanEntry::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        assert_eq!(game_dir.name, "Super Mario World");
        assert_eq!(game_dir.entries.len(), 1);
    }

    #[test]
    fn presents_mixed_content_in_library_view() {
        let presenter = GroupedPresenter;
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

        let root = render_root(&presenter, &content, &policy);

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["megadrive", "ps1", "bios.bin", "manifest.txt"]);
    }

    #[test]
    fn presents_multi_disc_game_as_directory_with_playlist_in_library_view() {
        let presenter = GroupedPresenter;
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

        let root = render_root(&presenter, &content, &policy);

        assert_eq!(root.children()[0].name(), "ps1");

        let ps1 = match &root.children()[0] {
            crate::core::vfs::VfsNode::Directory(dir) => dir,
            other => panic!("expected directory, got {other:?}"),
        };

        let game_dir = match &ps1.children()[0] {
            crate::core::vfs::VfsNode::Directory(dir) => dir,
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
            crate::core::vfs::VfsNode::File(file) => file,
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
    fn naming_policy_can_change_grouped_output_names_without_changing_structure() {
        let presenter = GroupedPresenter;
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

        let default_root = render_root(&presenter, &content, &default_policy);
        let alt_root = render_root(&presenter, &content, &alt_policy);

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

    #[test]
    fn conflict_policy_can_disambiguate_game_directory_names_under_same_platform() {
        let presenter = GroupedPresenter;
        let policy = suffix_conflict_policy();

        let content = vec![
            NormalizedContent::Game(GameContent {
                id: ContentId::new("game-1"),
                source: SourceRef::new("file:/roms/game-1.bin"),
                title: "Duplicate Game".to_string(),
                platform: Platform::Ps1,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/game-1.bin"),
                    size: 100,
                })],
                consumed_sources: vec![],
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("game-2"),
                source: SourceRef::new("file:/roms/game-2.bin"),
                title: "Duplicate Game".to_string(),
                platform: Platform::Ps1,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/game-2.bin"),
                    size: 200,
                })],
                consumed_sources: vec![],
            }),
        ];

        let root = render_root(&presenter, &content, &policy);
        let ps1 = root.find_directory("ps1").unwrap();

        let names: Vec<&str> = ps1.children().iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["Duplicate Game", "Duplicate Game (1)"]);
    }
}
