#[cfg(test)]
mod flat_parity_tests {
    use retromount::core::content::{
        BytesContent, ContentId, DiscPart, GameContent, GamePart, NormalizedContent, Platform,
        TextContent,
    };
    use retromount::core::source::SourceRef;
    use retromount::output::capabilities::{ContentType, Format};
    use retromount::output::flat_presenter::FlatPresenter;
    use retromount::output::plan::{PlanEntry, PresentationPlan};
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

    #[test]
    fn single_disc_game_matches_flat_presenter() {
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

        // Old path
        let presenter_plan = FlatPresenter::new().present(&content, &policy);

        // New path
        let spec = flat_spec_for_single_disc();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_plan_equivalent(&presenter_plan, &compiled_plan);
    }

    fn assert_plan_equivalent(expected: &PresentationPlan, actual: &PresentationPlan) {
        assert_eq!(expected.entries.len(), actual.entries.len());

        for (expected_entry, actual_entry) in expected.entries.iter().zip(actual.entries.iter()) {
            match (expected_entry, actual_entry) {
                (PlanEntry::File(expected_file), PlanEntry::File(actual_file)) => {
                    println!("expected name: {}", expected_file.name);
                    println!(
                        "expected content_type: {:?}",
                        expected_file.artifact.requirements.content_type
                    );
                    println!(
                        "expected format: {:?}",
                        expected_file.artifact.requirements.format
                    );

                    println!("actual name: {}", actual_file.name);
                    println!(
                        "actual content_type: {:?}",
                        actual_file.artifact.requirements.content_type
                    );
                    println!(
                        "actual format: {:?}",
                        actual_file.artifact.requirements.format
                    );

                    assert_eq!(expected_file.name, actual_file.name);

                    assert_eq!(
                        expected_file.artifact.requirements.content_type,
                        actual_file.artifact.requirements.content_type
                    );

                    assert_eq!(
                        expected_file.artifact.requirements.format,
                        actual_file.artifact.requirements.format
                    );
                }
                _ => panic!("expected matching file entries"),
            }
        }
    }

    #[test]
    fn bytes_content_matches_flat_presenter() {
        let content = vec![NormalizedContent::Bytes(BytesContent {
            id: ContentId::new("bytes:manual"),
            source: SourceRef::new("file:/roms/manual"),
            size: 123,
        })];

        let policy = test_policy();

        let presenter_plan = FlatPresenter::new().present(&content, &policy);

        let spec = flat_spec_for_bytes();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_plan_equivalent(&presenter_plan, &compiled_plan);
    }

    #[test]
    fn text_content_matches_flat_presenter() {
        let content = vec![NormalizedContent::Text(TextContent {
            id: ContentId::new("text:readme"),
            source: SourceRef::new("file:/roms/readme"),
            size: 42,
        })];

        let policy = test_policy();

        let presenter_plan = FlatPresenter::new().present(&content, &policy);

        let spec = flat_spec_for_text();
        let compiled_plan = compile_presentation_spec(&spec, &content, &policy);

        assert_plan_equivalent(&presenter_plan, &compiled_plan);
    }
}
