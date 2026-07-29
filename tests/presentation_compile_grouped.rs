#[cfg(test)]
mod grouped_parity_tests {
    use retromount::core::content::{
        ContentId, DiscPart, GameContent, GamePart, NormalizedContent, Platform,
    };
    use retromount::core::source::SourceRef;
    use retromount::output::capabilities::{ContentType, Format};
    use retromount::output::grouped_presenter::GroupedPresenter;
    use retromount::output::plan::{PlanDirectory, PlanEntry, PresentationPlan};
    use retromount::output::present::OutputPresenter;
    use retromount::output::presentation_compile::compile_presentation_spec;
    use retromount::output::presentation_spec::{
        ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
    };
    use retromount::policy::{
        default::{DefaultConflictPolicy, DefaultFormattingPolicy, DefaultNamingPolicy},
        PolicySet,
    };

    fn test_policy() -> PolicySet {
        PolicySet::new(
            Box::new(DefaultNamingPolicy),
            Box::new(DefaultFormattingPolicy),
            Box::new(DefaultConflictPolicy),
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

    #[test]
    fn single_disc_game_matches_grouped_presenter() {
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

        let presenter_plan = GroupedPresenter::new().present(&content, &policy);

        let spec = grouped_spec_for_single_disc();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_grouped_plan_equivalent(&presenter_plan, &compiled_plan);
    }

    #[test]
    fn multi_disc_game_with_playlist_matches_grouped_presenter() {
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
        let presenter_plan = GroupedPresenter::new().present(&content, &policy);
        let compiled_plan =
            compile_presentation_spec(&grouped_spec_for_multi_disc(), &content, &policy);

        assert_grouped_plan_equivalent(&presenter_plan, &compiled_plan);
    }

    fn assert_grouped_plan_equivalent(expected: &PresentationPlan, actual: &PresentationPlan) {
        assert_eq!(expected.entries.len(), actual.entries.len());

        for (expected_entry, actual_entry) in expected.entries.iter().zip(actual.entries.iter()) {
            assert_entry_equivalent(expected_entry, actual_entry);
        }
    }

    fn assert_entry_equivalent(expected: &PlanEntry, actual: &PlanEntry) {
        match (expected, actual) {
            (PlanEntry::Directory(expected_dir), PlanEntry::Directory(actual_dir)) => {
                assert_directory_equivalent(expected_dir, actual_dir);
            }
            (PlanEntry::File(expected_file), PlanEntry::File(actual_file)) => {
                assert_eq!(expected_file.name, actual_file.name);
                assert_eq!(expected_file.artifact, actual_file.artifact);
            }
            _ => panic!("expected matching entry kinds"),
        }
    }

    fn assert_directory_equivalent(expected: &PlanDirectory, actual: &PlanDirectory) {
        assert_eq!(expected.name, actual.name);
        assert_eq!(expected.entries.len(), actual.entries.len());

        for (expected_entry, actual_entry) in expected.entries.iter().zip(actual.entries.iter()) {
            assert_entry_equivalent(expected_entry, actual_entry);
        }
    }
}
