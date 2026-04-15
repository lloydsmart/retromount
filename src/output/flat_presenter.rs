use crate::core::content::{
    BytesContent, DiscPart, GameContent, GamePart, NormalizedContent, TextContent,
};
use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
use crate::output::plan::{
    ArtifactId, ArtifactReference, ArtifactRequest, GeneratedArtifact, PlanEntry, PlanFile,
    PlannedArtifactKind, PlaylistArtifact, PresentationPlan, SourceArtifact,
};
use crate::output::present::OutputPresenter;
use crate::output::presentation_expansion::{is_multi_disc_game, sorted_disc_parts};
use crate::policy::PolicySet;

pub struct FlatPresenter;

impl FlatPresenter {
    pub fn new() -> Self {
        Self
    }

    fn build_root_entries(
        &self,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> Vec<PlanEntry> {
        let mut root_names: Vec<String> = Vec::new();
        let mut entries = Vec::new();

        for item in content {
            match item {
                NormalizedContent::Game(game) => {
                    self.add_game_entries(game, policy, &mut root_names, &mut entries);
                }
                NormalizedContent::Bytes(bytes) => {
                    let proposed_name = self.bytes_file_name(bytes);
                    let allocated_name = policy.resolve_name_conflict(&proposed_name, &root_names);
                    root_names.push(allocated_name.clone());

                    entries.push(PlanEntry::File(PlanFile::new(
                        allocated_name,
                        ArtifactRequest::new(
                            ArtifactId::new(format!("bytes:{}", bytes.id)),
                            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                                bytes.source.clone(),
                                bytes.size,
                            )),
                            CapabilityRequirements::new(ContentType::Bytes)
                                .with_format(Format::Bin),
                        ),
                    )));
                }
                NormalizedContent::Text(text) => {
                    let proposed_name = self.text_file_name(text);
                    let allocated_name = policy.resolve_name_conflict(&proposed_name, &root_names);
                    root_names.push(allocated_name.clone());

                    entries.push(PlanEntry::File(PlanFile::new(
                        allocated_name,
                        ArtifactRequest::new(
                            ArtifactId::new(format!("text:{}", text.id)),
                            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                                text.source.clone(),
                                text.size,
                            )),
                            CapabilityRequirements::new(ContentType::Text)
                                .with_format(Format::Text),
                        ),
                    )));
                }
            }
        }

        entries
    }

    fn add_game_entries(
        &self,
        game: &GameContent,
        policy: &PolicySet,
        root_names: &mut Vec<String>,
        entries: &mut Vec<PlanEntry>,
    ) {
        if is_multi_disc_game(game) {
            self.add_multi_disc_game_entries(game, policy, root_names, entries);
        } else {
            let proposed_name = match game.parts.first() {
                Some(part) => policy.naming().part_name(game, part),
                None => policy.naming().game_name(game),
            };

            let allocated_name = policy.resolve_name_conflict(&proposed_name, root_names);
            root_names.push(allocated_name.clone());

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
                    PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                        game.source.clone(),
                        0,
                    )),
                    CapabilityRequirements::new(ContentType::Game),
                ),
            };

            entries.push(PlanEntry::File(PlanFile::new(allocated_name, artifact)));
        }
    }

    fn add_multi_disc_game_entries(
        &self,
        game: &GameContent,
        policy: &PolicySet,
        root_names: &mut Vec<String>,
        entries: &mut Vec<PlanEntry>,
    ) {
        let mut disc_refs = Vec::new();

        for disc in sorted_disc_parts(game) {
            let proposed_name = policy
                .naming()
                .part_name(game, &GamePart::Disc(disc.clone()));
            let allocated_name = policy.resolve_name_conflict(&proposed_name, root_names);
            root_names.push(allocated_name.clone());

            let artifact_id =
                ArtifactId::new(format!("game:{}:disc:{}", game.id, disc.disc_number));
            disc_refs.push(ArtifactReference::new(artifact_id.clone()));

            entries.push(PlanEntry::File(PlanFile::new(
                allocated_name,
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
            policy.resolve_name_conflict(&policy.naming().playlist_name(game), root_names);
        root_names.push(playlist_name.clone());

        entries.push(PlanEntry::File(PlanFile::new(
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

    fn bytes_file_name(&self, bytes: &BytesContent) -> String {
        format!("{}.bin", bytes.source.file_name())
    }

    fn text_file_name(&self, text: &TextContent) -> String {
        let file_name = text.source.file_name();

        if file_name.contains('.') {
            file_name
        } else {
            format!("{file_name}.txt")
        }
    }
}

impl Default for FlatPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputPresenter for FlatPresenter {
    fn present(&self, content: &[NormalizedContent], policy: &PolicySet) -> PresentationPlan {
        PresentationPlan::new(self.build_root_entries(content, policy))
    }
}

fn estimate_disc_size(_game: &GameContent, _disc: &DiscPart) -> u64 {
    // TODO(phase5b): carry authoritative source-backed sizes for disc artifacts
    // through the presentation plan. Disc cues currently materialize with size 0.
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, ContentId, DiscPart, GameContent, GamePart, Platform, RomPart, TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::output::materialize::materialize_plan;
    use crate::policy::{ConflictPolicy, FormattingPolicy, NamingPolicy, PolicySet};

    struct DefaultLikeNamingPolicy;

    fn render_root(
        presenter: &FlatPresenter,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> crate::core::vfs::VfsDirectory {
        let plan = presenter.present(content, policy);
        materialize_plan(&plan).unwrap()
    }

    fn render_plan(
        presenter: &FlatPresenter,
        content: &[NormalizedContent],
        policy: &PolicySet,
    ) -> PresentationPlan {
        presenter.present(content, policy)
    }

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

    fn plan_file_names(plan: &PresentationPlan) -> Vec<&str> {
        plan.entries
            .iter()
            .map(|entry| match entry {
                PlanEntry::File(file) => file.name.as_str(),
                PlanEntry::Directory(dir) => dir.name.as_str(),
            })
            .collect()
    }

    #[test]
    fn presents_mixed_content_at_root() {
        let presenter = FlatPresenter;
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
                source: SourceRef::new("file:/roms/manifest.txt"),
                size: 64,
            }),
        ];

        let plan = render_plan(&presenter, &content, &policy);

        assert_eq!(
            plan_file_names(&plan),
            vec![
                "bios.bin",
                "sonic.bin",
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
                "manifest.txt",
            ]
        );
    }

    #[test]
    fn conflict_policy_can_disambiguate_playlist_between_duplicate_multi_disc_games() {
        let presenter = FlatPresenter;
        let policy = suffix_conflict_policy();

        let content = vec![
            NormalizedContent::Game(GameContent {
                id: ContentId::new("ff7-a"),
                source: SourceRef::new("cue:/roms/ff7-a-disc1.cue"),
                title: "Final Fantasy VII".to_string(),
                platform: Platform::Ps1,
                parts: vec![
                    GamePart::Disc(DiscPart {
                        source: SourceRef::new("cue:/roms/ff7-a-disc2.cue"),
                        disc_number: 2,
                        consumed_sources: vec![SourceRef::new("cue:/roms/ff7-a-disc2.bin")],
                    }),
                    GamePart::Disc(DiscPart {
                        source: SourceRef::new("cue:/roms/ff7-a-disc1.cue"),
                        disc_number: 1,
                        consumed_sources: vec![SourceRef::new("cue:/roms/ff7-a-disc1.bin")],
                    }),
                ],
                consumed_sources: vec![
                    SourceRef::new("cue:/roms/ff7-a-disc2.bin"),
                    SourceRef::new("cue:/roms/ff7-a-disc1.bin"),
                ],
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("ff7-b"),
                source: SourceRef::new("cue:/roms/ff7-b-disc1.cue"),
                title: "Final Fantasy VII".to_string(),
                platform: Platform::Ps1,
                parts: vec![
                    GamePart::Disc(DiscPart {
                        source: SourceRef::new("cue:/roms/ff7-b-disc2.cue"),
                        disc_number: 2,
                        consumed_sources: vec![SourceRef::new("cue:/roms/ff7-b-disc2.bin")],
                    }),
                    GamePart::Disc(DiscPart {
                        source: SourceRef::new("cue:/roms/ff7-b-disc1.cue"),
                        disc_number: 1,
                        consumed_sources: vec![SourceRef::new("cue:/roms/ff7-b-disc1.bin")],
                    }),
                ],
                consumed_sources: vec![
                    SourceRef::new("cue:/roms/ff7-b-disc2.bin"),
                    SourceRef::new("cue:/roms/ff7-b-disc1.bin"),
                ],
            }),
        ];

        let plan = render_plan(&presenter, &content, &policy);

        assert_eq!(
            plan_file_names(&plan),
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
                "Final Fantasy VII (Disc 1).cue (1)",
                "Final Fantasy VII (Disc 2).cue (1)",
                "Final Fantasy VII.m3u (1)",
            ]
        );
    }

    #[test]
    fn multi_disc_plan_emits_playlist_artifact_referencing_disc_artifacts() {
        let presenter = FlatPresenter;
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

        let plan = render_plan(&presenter, &content, &policy);

        assert_eq!(plan.entries.len(), 3);

        let playlist = match &plan.entries[2] {
            PlanEntry::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &playlist.artifact.kind {
            crate::output::plan::PlannedArtifactKind::Generated(
                crate::output::plan::GeneratedArtifact::Playlist(playlist),
            ) => {
                assert_eq!(playlist.entries.len(), 2);
                assert_eq!(playlist.entries[0].artifact_id.0, "game:ff7:disc:1");
                assert_eq!(playlist.entries[1].artifact_id.0, "game:ff7:disc:2");
            }
            other => panic!("expected playlist artifact, got {other:?}"),
        }
    }

    #[test]
    fn materialized_multi_disc_output_contains_playlist_contents() {
        let presenter = FlatPresenter;
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

        let names: Vec<&str> = root.children().iter().map(|node| node.name()).collect();
        assert_eq!(
            names,
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ]
        );
    }
}
