#[cfg(test)]
mod grouped_parity_tests {
    use retromount::core::content::{
        BytesContent, ContentId, DiscPart, GameContent, GamePart, NormalizedContent, Platform,
        RomPart, TextContent,
    };
    use retromount::core::source::SourceRef;
    use retromount::output::capabilities::{ContentType, Format};
    use retromount::output::plan::{PlanEntry, PresentationPlan};
    use retromount::output::presentation_compile::compile_presentation_spec;
    use retromount::output::presentation_spec::{
        ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
    };
    use retromount::policy::{
        default::{DefaultConflictPolicy, DefaultFormattingPolicy, DefaultNamingPolicy},
        ConflictPolicy, PolicySet,
    };

    struct SuffixConflictPolicy;

    impl ConflictPolicy for SuffixConflictPolicy {
        fn resolve_name_conflict(&self, proposed: &str, existing: &[String]) -> String {
            if !existing.iter().any(|name| name == proposed) {
                return proposed.to_string();
            }

            let mut suffix = 1;
            loop {
                let candidate = format!("{proposed} ({suffix})");
                if !existing.iter().any(|name| name == &candidate) {
                    return candidate;
                }
                suffix += 1;
            }
        }
    }

    fn test_policy() -> PolicySet {
        PolicySet::new(
            Box::new(DefaultNamingPolicy),
            Box::new(DefaultFormattingPolicy),
            Box::new(DefaultConflictPolicy),
        )
    }

    fn suffix_conflict_policy() -> PolicySet {
        PolicySet::new(
            Box::new(DefaultNamingPolicy),
            Box::new(DefaultFormattingPolicy),
            Box::new(SuffixConflictPolicy),
        )
    }

    fn grouped_spec_for_single_disc() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::GroupedByPlatformAndGame,
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGames,
                NamingSpec::PartName,
                ArtifactSpec::new(ContentType::Disc).with_format(Format::Bin),
            )],
        )
    }

    fn grouped_spec_for_multi_disc() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::GroupedByPlatformAndGame,
            vec![
                FileRuleSpec::new(
                    SelectSpec::MultiDiscGames,
                    NamingSpec::PartName,
                    ArtifactSpec::new(ContentType::Disc).with_format(Format::Bin),
                ),
                FileRuleSpec::new(
                    SelectSpec::MultiDiscGames,
                    NamingSpec::PlaylistName,
                    ArtifactSpec::new(ContentType::Playlist).with_format(Format::M3u),
                ),
            ],
        )
    }

    fn grouped_spec_for_mixed_content() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::GroupedByPlatformAndGame,
            vec![
                FileRuleSpec::new(
                    SelectSpec::SingleRomGames,
                    NamingSpec::PartName,
                    ArtifactSpec::new(ContentType::Rom).with_format(Format::Bin),
                ),
                FileRuleSpec::new(
                    SelectSpec::SingleDiscGames,
                    NamingSpec::PartName,
                    ArtifactSpec::new(ContentType::Disc).with_format(Format::Bin),
                ),
                FileRuleSpec::new(
                    SelectSpec::Bytes,
                    NamingSpec::SourceName,
                    ArtifactSpec::new(ContentType::Bytes).with_format(Format::Bin),
                ),
                FileRuleSpec::new(
                    SelectSpec::Text,
                    NamingSpec::SourceName,
                    ArtifactSpec::new(ContentType::Text).with_format(Format::Text),
                ),
            ],
        )
    }

    #[test]
    fn single_disc_game_has_expected_grouped_output() {
        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ps2:ico"),
            source: SourceRef::new("file:/roms/ico.chd"),
            title: "ICO".to_string(),
            platform: Platform::Unknown,
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("file:/roms/ico.chd"),
                disc_number: 1,
                consumed_sources: vec![],
            })],
            consumed_sources: vec![],
        })];

        let policy = test_policy();

        let spec = grouped_spec_for_single_disc();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_plan_paths(
            &compiled_plan,
            &["unknown/", "unknown/ICO/", "unknown/ICO/ICO (Disc 1).cue"],
        );
    }

    #[test]
    fn multi_disc_game_with_playlist_has_expected_grouped_output() {
        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ps1:final-fantasy-vii"),
            source: SourceRef::new("file:/roms/Final Fantasy VII (Disc 1).cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("file:/roms/Final Fantasy VII (Disc 2).cue"),
                    disc_number: 2,
                    consumed_sources: vec![],
                }),
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("file:/roms/Final Fantasy VII (Disc 1).cue"),
                    disc_number: 1,
                    consumed_sources: vec![],
                }),
            ],
            consumed_sources: vec![],
        })];

        let policy = test_policy();
        let compiled_plan =
            compile_presentation_spec(&grouped_spec_for_multi_disc(), &content, &policy);

        assert_plan_paths(
            &compiled_plan,
            &[
                "ps1/",
                "ps1/Final Fantasy VII/",
                "ps1/Final Fantasy VII/Final Fantasy VII (Disc 1).cue",
                "ps1/Final Fantasy VII/Final Fantasy VII (Disc 2).cue",
                "ps1/Final Fantasy VII/Final Fantasy VII.m3u",
            ],
        );
    }

    #[test]
    fn mixed_content_with_preserved_paths_has_expected_grouped_output() {
        let content = vec![
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new(r"firmware\ps1\bios"),
                source: SourceRef::new("file:/roms/firmware/ps1/scph1001"),
                size: 512,
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("megadrive:sonic"),
                source: SourceRef::new("file:/roms/megadrive/sonic.bin"),
                title: "Sonic the Hedgehog".to_string(),
                platform: Platform::Megadrive,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/megadrive/sonic.bin"),
                    size: 1024,
                })],
                consumed_sources: vec![],
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("megadrive:streets-of-rage"),
                source: SourceRef::new("file:/roms/megadrive/streets-of-rage.bin"),
                title: "Streets of Rage".to_string(),
                platform: Platform::Megadrive,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/megadrive/streets-of-rage.bin"),
                    size: 2048,
                })],
                consumed_sources: vec![],
            }),
            NormalizedContent::Text(TextContent {
                id: ContentId::new("docs/guides/readme"),
                source: SourceRef::new("file:/roms/docs/guides/readme"),
                size: 64,
            }),
        ];

        let policy = test_policy();
        let compiled_plan =
            compile_presentation_spec(&grouped_spec_for_mixed_content(), &content, &policy);

        assert_plan_paths(
            &compiled_plan,
            &[
                "firmware/",
                "firmware/ps1/",
                "firmware/ps1/scph1001.bin",
                "megadrive/",
                "megadrive/Sonic the Hedgehog/",
                "megadrive/Sonic the Hedgehog/sonic.bin",
                "megadrive/Streets of Rage/",
                "megadrive/Streets of Rage/streets-of-rage.bin",
                "docs/",
                "docs/guides/",
                "docs/guides/readme.txt",
            ],
        );
    }

    #[test]
    fn duplicate_game_directories_apply_grouped_conflict_handling() {
        let content = vec![
            NormalizedContent::Game(GameContent {
                id: ContentId::new("ps1:duplicate-one"),
                source: SourceRef::new("file:/roms/duplicate-one.bin"),
                title: "Duplicate Game".to_string(),
                platform: Platform::Ps1,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/duplicate-one.bin"),
                    size: 100,
                })],
                consumed_sources: vec![],
            }),
            NormalizedContent::Game(GameContent {
                id: ContentId::new("ps1:duplicate-two"),
                source: SourceRef::new("file:/roms/duplicate-two.bin"),
                title: "Duplicate Game".to_string(),
                platform: Platform::Ps1,
                parts: vec![GamePart::Rom(RomPart {
                    source: SourceRef::new("file:/roms/duplicate-two.bin"),
                    size: 200,
                })],
                consumed_sources: vec![],
            }),
        ];

        let policy = suffix_conflict_policy();
        let compiled_plan =
            compile_presentation_spec(&grouped_spec_for_mixed_content(), &content, &policy);

        assert_plan_paths(
            &compiled_plan,
            &[
                "ps1/",
                "ps1/Duplicate Game/",
                "ps1/Duplicate Game/duplicate-one.bin",
                "ps1/Duplicate Game (1)/",
                "ps1/Duplicate Game (1)/duplicate-two.bin",
            ],
        );
    }

    #[test]
    fn colliding_preserved_path_files_apply_grouped_conflict_handling() {
        let content = vec![
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("firmware/primary"),
                source: SourceRef::new("file:/roms/primary/bios"),
                size: 512,
            }),
            NormalizedContent::Bytes(BytesContent {
                id: ContentId::new("firmware/backup"),
                source: SourceRef::new("file:/roms/backup/bios"),
                size: 512,
            }),
        ];

        let policy = suffix_conflict_policy();
        let compiled_plan =
            compile_presentation_spec(&grouped_spec_for_mixed_content(), &content, &policy);

        assert_plan_paths(
            &compiled_plan,
            &["firmware/", "firmware/bios.bin", "firmware/bios.bin (1)"],
        );
    }

    fn assert_plan_paths(plan: &PresentationPlan, expected: &[&str]) {
        let mut actual = Vec::new();
        collect_paths(&plan.entries, "", &mut actual);
        let expected: Vec<String> = expected.iter().map(|path| path.to_string()).collect();
        assert_eq!(actual, expected);
    }

    fn collect_paths(entries: &[PlanEntry], parent: &str, paths: &mut Vec<String>) {
        for entry in entries {
            match entry {
                PlanEntry::Directory(directory) => {
                    let path = format!("{parent}{}/", directory.name);
                    paths.push(path.clone());
                    collect_paths(&directory.entries, &path, paths);
                }
                PlanEntry::File(file) => {
                    let path = format!("{parent}{}", file.name);
                    paths.push(path);
                }
            }
        }
    }
}
