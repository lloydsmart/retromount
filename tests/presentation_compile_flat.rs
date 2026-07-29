#[cfg(test)]
mod flat_parity_tests {
    use retromount::core::content::{
        BytesContent, ContentId, DiscPart, GameContent, GamePart, NormalizedContent, Platform,
        TextContent,
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

    fn flat_spec_for_single_disc() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGames,
                NamingSpec::PartName,
                ArtifactSpec::new(ContentType::Disc).with_format(Format::Bin),
            )],
        )
    }

    fn flat_spec_for_bytes() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::Bytes,
                NamingSpec::SourceName,
                ArtifactSpec::new(ContentType::Bytes).with_format(Format::Bin),
            )],
        )
    }

    fn flat_spec_for_text() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::Text,
                NamingSpec::SourceName,
                ArtifactSpec::new(ContentType::Text).with_format(Format::Text),
            )],
        )
    }

    fn flat_spec_for_multi_disc() -> PresentationSpec {
        PresentationSpec::new(
            LayoutSpec::Flat,
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

    fn multi_disc_game(id: &str) -> NormalizedContent {
        NormalizedContent::Game(GameContent {
            id: ContentId::new(id),
            source: SourceRef::new(format!("file:/roms/{id}-disc1.cue")),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![
                GamePart::Disc(DiscPart {
                    source: SourceRef::new(format!("file:/roms/{id}-disc2.cue")),
                    disc_number: 2,
                    consumed_sources: vec![],
                }),
                GamePart::Disc(DiscPart {
                    source: SourceRef::new(format!("file:/roms/{id}-disc1.cue")),
                    disc_number: 1,
                    consumed_sources: vec![],
                }),
            ],
            consumed_sources: vec![],
        })
    }

    #[test]
    fn single_disc_game_has_expected_flat_output() {
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

        let spec = flat_spec_for_single_disc();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_file_names(&compiled_plan, &["ICO (Disc 1).cue"]);
    }

    fn assert_file_names(plan: &PresentationPlan, expected: &[&str]) {
        let actual: Vec<&str> = plan
            .entries
            .iter()
            .map(|entry| match entry {
                PlanEntry::File(file) => file.name.as_str(),
                PlanEntry::Directory(_) => panic!("flat plan should contain only files"),
            })
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn bytes_content_has_expected_flat_output() {
        let content = vec![NormalizedContent::Bytes(BytesContent {
            id: ContentId::new("bytes:manual"),
            source: SourceRef::new("file:/roms/manual"),
            size: 123,
        })];

        let policy = test_policy();

        let spec = flat_spec_for_bytes();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_file_names(&compiled_plan, &["manual.bin"]);
    }

    #[test]
    fn text_content_has_expected_flat_output() {
        let content = vec![NormalizedContent::Text(TextContent {
            id: ContentId::new("text:readme"),
            source: SourceRef::new("file:/roms/readme"),
            size: 42,
        })];

        let policy = test_policy();

        let spec = flat_spec_for_text();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_file_names(&compiled_plan, &["readme.txt"]);
    }

    #[test]
    fn multi_disc_game_with_playlist_has_expected_flat_output() {
        let content = vec![multi_disc_game("ff7")];
        let policy = test_policy();

        let compiled_plan =
            compile_presentation_spec(&flat_spec_for_multi_disc(), &content, &policy);

        assert_file_names(
            &compiled_plan,
            &[
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ],
        );

        let PlanEntry::File(playlist) = &compiled_plan.entries[2] else {
            panic!("expected playlist file");
        };
        let retromount::output::plan::PlannedArtifactKind::Generated(
            retromount::output::plan::GeneratedArtifact::Playlist(playlist),
        ) = &playlist.artifact.kind
        else {
            panic!("expected generated playlist artifact");
        };
        let references: Vec<&str> = playlist
            .entries
            .iter()
            .map(|entry| entry.artifact_id.0.as_str())
            .collect();
        assert_eq!(references, ["game:ff7:disc:1", "game:ff7:disc:2"]);
    }

    #[test]
    fn duplicate_multi_disc_games_apply_flat_conflict_handling() {
        let content = vec![multi_disc_game("ff7-a"), multi_disc_game("ff7-b")];
        let policy = suffix_conflict_policy();

        let compiled_plan =
            compile_presentation_spec(&flat_spec_for_multi_disc(), &content, &policy);

        assert_file_names(
            &compiled_plan,
            &[
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
                "Final Fantasy VII (Disc 1).cue (1)",
                "Final Fantasy VII (Disc 2).cue (1)",
                "Final Fantasy VII.m3u (1)",
            ],
        );
    }
}
