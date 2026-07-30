use crate::core::content::{DiscMedia, Platform};
use crate::output::capabilities::{CapabilityFeature, ContentType, Format};
use crate::output::presentation_spec::{
    ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
};

const BUILT_IN_PRESENTATIONS: [&str; 3] = ["flat", "grouped", "opl"];

pub fn built_in_presentation_names() -> &'static [&'static str] {
    &BUILT_IN_PRESENTATIONS
}

pub fn build_presentation_spec(name: &str) -> Result<PresentationSpec, String> {
    match name {
        "flat" => Ok(PresentationSpec::new(
            LayoutSpec::Flat,
            built_in_file_rules(),
        )),
        "grouped" => Ok(PresentationSpec::new(
            LayoutSpec::GroupedByPlatformAndGame,
            built_in_file_rules(),
        )),
        "opl" => Ok(PresentationSpec::new(
            LayoutSpec::LiteralRoot("DVD".to_string()),
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGamesByPlatformAndMedia {
                    platform: Platform::Ps2,
                    media: DiscMedia::Dvd,
                },
                NamingSpec::GameTitle,
                ArtifactSpec::new(ContentType::Disc)
                    .with_format(Format::Iso)
                    .require_feature(CapabilityFeature::RandomAccess)
                    .require_feature(CapabilityFeature::Lossless),
            )],
        )),
        _ => Err(format!(
            "unsupported view '{name}'; expected one of: {}",
            built_in_presentation_names().join(", ")
        )),
    }
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
        ContentId, DiscPart, GameContent, GamePart, LogicalDisc, NormalizedContent, Platform,
    };
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::SourceRef;
    use crate::core::vfs_reader::open_vfs_file;
    use crate::output::materialize::materialize_plan;
    use crate::output::plan::{GeneratedArtifact, PlanEntry, PlannedArtifactKind};
    use crate::output::presentation_compile::compile_presentation_spec;
    use crate::policy::PolicySet;
    use crate::readers::inline_reader::InlineReader;

    #[test]
    fn exposes_stable_built_in_presentation_names() {
        assert_eq!(built_in_presentation_names(), &["flat", "grouped", "opl"]);
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
        assert_eq!(
            build_presentation_spec("opl").unwrap().layout,
            LayoutSpec::LiteralRoot("DVD".to_string())
        );
    }

    #[test]
    fn rejects_unknown_built_in_presentation() {
        let error = build_presentation_spec("unknown").unwrap_err();
        assert_eq!(
            error,
            "unsupported view 'unknown'; expected one of: flat, grouped, opl"
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
                    logical_disc: None,
                }),
                GamePart::Disc(DiscPart {
                    source: SourceRef::new("file:/roms/ff7-disc1.cue"),
                    disc_number: 1,
                    consumed_sources: vec![],
                    logical_disc: None,
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

    #[test]
    fn opl_presentation_materializes_live_ps2_dvd_iso() {
        let disc_bytes = vec![0x5a; 4096];
        let reader_bytes = disc_bytes.clone();
        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ps2:game"),
            source: SourceRef::new("file:/roms/ps2/Game.chd"),
            title: "Game".to_string(),
            platform: Platform::Ps2,
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("file:/roms/ps2/Game.chd"),
                disc_number: 1,
                consumed_sources: vec![],
                logical_disc: Some(LogicalDisc {
                    media: DiscMedia::Dvd,
                    sector_size: 2048,
                    sector_count: 2,
                    content: ReaderHandle::new("test:opl-disc", move || {
                        Ok(Box::new(InlineReader::new(reader_bytes.clone())))
                    }),
                }),
            })],
            consumed_sources: vec![],
        })];

        let presentation = build_presentation_spec("opl").unwrap();
        let plan = compile_presentation_spec(&presentation, &content, &PolicySet::default());
        let root = materialize_plan(&plan).unwrap();
        let file = root.find_file("DVD/Game.iso").unwrap();
        let mut reader = open_vfs_file(file).unwrap();
        let mut bytes = [0; 8];

        assert_eq!(file.size, 4096);
        reader.read_at(2044, &mut bytes).unwrap();
        assert_eq!(&bytes, &[0x5a; 8]);
    }
}
