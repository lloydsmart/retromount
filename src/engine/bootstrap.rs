use crate::output::capabilities::{ContentType, Format};
use crate::output::presentation_spec::{
    ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
};

const BUILT_IN_PRESENTATIONS: [&str; 2] = ["flat", "grouped"];

pub fn built_in_presentation_names() -> &'static [&'static str] {
    &BUILT_IN_PRESENTATIONS
}

pub fn build_presentation_spec(name: &str) -> Result<PresentationSpec, String> {
    let layout = match name {
        "flat" => LayoutSpec::Flat,
        "grouped" => LayoutSpec::GroupedByPlatformAndGame,
        _ => {
            return Err(format!(
                "unsupported view '{name}'; expected one of: {}",
                built_in_presentation_names().join(", ")
            ))
        }
    };

    Ok(PresentationSpec::new(layout, built_in_file_rules()))
}

fn built_in_file_rules() -> Vec<FileRuleSpec> {
    vec![
        FileRuleSpec::new(
            SelectSpec::GamesWithoutParts,
            NamingSpec::GameName,
            ArtifactSpec::new(ContentType::Game),
        ),
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
            SelectSpec::MultiDiscGames,
            NamingSpec::PartName,
            ArtifactSpec::new(ContentType::Disc).with_format(Format::Bin),
        ),
        FileRuleSpec::new(
            SelectSpec::MultiDiscGames,
            NamingSpec::PlaylistName,
            ArtifactSpec::new(ContentType::Playlist).with_format(Format::M3u),
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        ContentId, DiscPart, GameContent, GamePart, NormalizedContent, Platform,
    };
    use crate::core::source::SourceRef;
    use crate::output::plan::{GeneratedArtifact, PlanEntry, PlannedArtifactKind};
    use crate::output::presentation_compile::compile_presentation_spec;
    use crate::policy::PolicySet;

    #[test]
    fn exposes_stable_built_in_presentation_names() {
        assert_eq!(built_in_presentation_names(), &["flat", "grouped"]);
    }

    #[test]
    fn builds_expected_layout_for_each_built_in_presentation() {
        assert_eq!(
            build_presentation_spec("flat").unwrap().layout,
            LayoutSpec::Flat
        );
        assert_eq!(
            build_presentation_spec("grouped").unwrap().layout,
            LayoutSpec::GroupedByPlatformAndGame
        );
    }

    #[test]
    fn rejects_unknown_built_in_presentation() {
        let error = build_presentation_spec("unknown").unwrap_err();
        assert_eq!(
            error,
            "unsupported view 'unknown'; expected one of: flat, grouped"
        );
    }

    #[test]
    fn compiled_flat_presentation_emits_multi_disc_playlist() {
        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("file:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("file:/roms/ff7-disc2.cue"),
                    disc_number: 2,
                    consumed_sources: vec![],
                }),
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("file:/roms/ff7-disc1.cue"),
                    disc_number: 1,
                    consumed_sources: vec![],
                }),
            ],
            consumed_sources: vec![],
        })];

        let presentation = build_presentation_spec("flat").unwrap();
        let plan = compile_presentation_spec(&presentation, &content, &PolicySet::default());

        assert_eq!(plan.entries.len(), 3);
        let PlanEntry::File(playlist) = &plan.entries[2] else {
            panic!("expected playlist file");
        };
        assert!(matches!(
            playlist.artifact.kind,
            PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(_))
        ));
    }
}
