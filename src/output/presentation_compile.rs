use crate::core::content::{ContentMeta, GameContent, GamePart, NormalizedContent};
use crate::output::capabilities::{CapabilityRequirements, Format};
use crate::output::plan::{
    ArtifactId, ArtifactRequest, PlanEntry, PlanFile, PlannedArtifactKind, PresentationPlan,
    SourceArtifact,
};
use crate::output::presentation_spec::{
    ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
};
use crate::policy::PolicySet;

/// Compiles a declarative `PresentationSpec` into a concrete `PresentationPlan`.
///
/// This initial compiler is intentionally minimal:
///
/// - supports flat layout only
/// - emits files only
/// - supports single-source artifacts only
/// - does not yet handle grouped layouts, playlists, or generated artifacts
pub fn compile_presentation_spec(
    spec: &PresentationSpec,
    content: &[NormalizedContent],
    policy: &PolicySet,
) -> PresentationPlan {
    match spec.layout {
        LayoutSpec::Flat => compile_flat(spec, content, policy),
    }
}

fn compile_flat(
    spec: &PresentationSpec,
    content: &[NormalizedContent],
    policy: &PolicySet,
) -> PresentationPlan {
    let mut entries = Vec::new();
    let mut root_names = Vec::new();

    for item in content {
        if let Some(file) = compile_item(spec, item, policy, &mut root_names) {
            entries.push(PlanEntry::File(file));
        }
    }

    PresentationPlan::new(entries)
}

fn compile_item(
    spec: &PresentationSpec,
    item: &NormalizedContent,
    policy: &PolicySet,
    root_names: &mut Vec<String>,
) -> Option<PlanFile> {
    for rule in &spec.files {
        if let Some(file) = try_compile_rule(rule, item, policy, root_names) {
            return Some(file);
        }
    }

    None
}

fn try_compile_rule(
    rule: &FileRuleSpec,
    item: &NormalizedContent,
    policy: &PolicySet,
    root_names: &mut Vec<String>,
) -> Option<PlanFile> {
    let match_result = match rule.select {
        SelectSpec::Games => match_game(item),
        SelectSpec::SingleDiscGames => match_single_disc_game(item),
        SelectSpec::SingleRomGames => match_single_rom_game(item),
        SelectSpec::Bytes => match_bytes(item),
        SelectSpec::Text => match_text(item),
    }?;

    let proposed_name = build_name(&rule.naming, item, &rule.artifact)?;
    let proposed_name = apply_extension(proposed_name, rule.artifact.format);
    let allocated_name = policy.resolve_name_conflict(&proposed_name, root_names);
    root_names.push(allocated_name.clone());

    let artifact = ArtifactRequest::new(
        build_artifact_id(&rule.select, item),
        PlannedArtifactKind::SourceBacked(SourceArtifact::single(
            match_result.source,
            match_result.size,
        )),
        build_requirements(&rule.artifact),
    );

    Some(PlanFile::new(allocated_name, artifact))
}

struct SourceMatch {
    source: crate::core::source::SourceRef,
    size: u64,
}

fn match_game(item: &NormalizedContent) -> Option<SourceMatch> {
    match item {
        NormalizedContent::Game(game) => Some(SourceMatch {
            source: game.source.clone(),
            size: 0,
        }),
        _ => None,
    }
}

fn match_single_disc_game(item: &NormalizedContent) -> Option<SourceMatch> {
    let NormalizedContent::Game(game) = item else {
        return None;
    };

    if game.parts.len() != 1 {
        return None;
    }

    match &game.parts[0] {
        GamePart::Disc(disc) => Some(SourceMatch {
            source: disc.source.clone(),
            // Current presenter code also lacks authoritative disc size here.
            size: 0,
        }),
        GamePart::Rom(_) => None,
    }
}

fn match_single_rom_game(item: &NormalizedContent) -> Option<SourceMatch> {
    let NormalizedContent::Game(game) = item else {
        return None;
    };

    if game.parts.len() != 1 {
        return None;
    }

    match &game.parts[0] {
        GamePart::Rom(rom) => Some(SourceMatch {
            source: rom.source.clone(),
            size: rom.size,
        }),
        GamePart::Disc(_) => None,
    }
}

fn match_bytes(item: &NormalizedContent) -> Option<SourceMatch> {
    match item {
        NormalizedContent::Bytes(bytes) => Some(SourceMatch {
            source: bytes.source.clone(),
            size: bytes.size,
        }),
        _ => None,
    }
}

fn match_text(item: &NormalizedContent) -> Option<SourceMatch> {
    match item {
        NormalizedContent::Text(text) => Some(SourceMatch {
            source: text.source.clone(),
            size: text.size,
        }),
        _ => None,
    }
}

fn build_name(
    naming: &NamingSpec,
    item: &NormalizedContent,
    artifact: &ArtifactSpec,
) -> Option<String> {
    match naming {
        NamingSpec::GameTitle => game_title(item),
        NamingSpec::SourceName => source_name(item, artifact),
        NamingSpec::Literal(value) => Some(value.clone()),
    }
}

fn game_title(item: &NormalizedContent) -> Option<String> {
    match item {
        NormalizedContent::Game(game) => Some(game.title.clone()),
        _ => None,
    }
}

fn source_name(item: &NormalizedContent, artifact: &ArtifactSpec) -> Option<String> {
    let file_name = match item {
        NormalizedContent::Bytes(bytes) => bytes.source.file_name(),
        NormalizedContent::Text(text) => text.source.file_name(),
        NormalizedContent::Game(game) => source_name_for_game(game)?,
    };

    Some(strip_known_extension(&file_name, artifact.format))
}

fn source_name_for_game(game: &GameContent) -> Option<String> {
    if game.parts.len() != 1 {
        return None;
    }

    match &game.parts[0] {
        GamePart::Rom(rom) => Some(rom.source.file_name()),
        GamePart::Disc(disc) => Some(disc.source.file_name()),
    }
}

fn strip_known_extension(file_name: &str, format: Option<Format>) -> String {
    let expected_extension = extension_for_format(format);

    match expected_extension {
        Some(ext) if file_name.ends_with(&format!(".{ext}")) => file_name
            .strip_suffix(&format!(".{ext}"))
            .unwrap_or(file_name)
            .to_string(),
        _ => file_name.to_string(),
    }
}

fn apply_extension(name: String, format: Option<Format>) -> String {
    match extension_for_format(format) {
        Some(ext) if !name.ends_with(&format!(".{ext}")) => format!("{name}.{ext}"),
        _ => name,
    }
}

fn extension_for_format(format: Option<Format>) -> Option<&'static str> {
    match format {
        Some(Format::Iso) => Some("iso"),
        Some(Format::Chd) => Some("chd"),
        Some(Format::Zip) => Some("zip"),
        Some(Format::M3u) => Some("m3u"),
        Some(Format::Directory) => None,
        Some(Format::Bin) => Some("bin"),
        Some(Format::Text) => Some("txt"),
        None => None,
    }
}

fn build_artifact_id(select: &SelectSpec, item: &NormalizedContent) -> ArtifactId {
    match (select, item) {
        (SelectSpec::Games, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
        }
        (SelectSpec::SingleDiscGames, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}:disc", game.id))
        }
        (SelectSpec::SingleRomGames, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}:rom", game.id))
        }
        (SelectSpec::Bytes, NormalizedContent::Bytes(bytes)) => {
            ArtifactId::new(format!("bytes:{}", bytes.id))
        }
        (SelectSpec::Text, NormalizedContent::Text(text)) => {
            ArtifactId::new(format!("text:{}", text.id))
        }
        _ => ArtifactId::new(format!("content:{}", item.id())),
    }
}

fn build_requirements(spec: &ArtifactSpec) -> CapabilityRequirements {
    let mut requirements = CapabilityRequirements::new(spec.content_type);

    if let Some(format) = spec.format {
        requirements = requirements.with_format(format);
    }

    for feature in &spec.required_features {
        requirements = requirements.require_feature(*feature);
    }

    for feature in &spec.preferred_features {
        requirements = requirements.prefer_feature(*feature);
    }

    for feature in &spec.forbidden_features {
        requirements = requirements.forbid_feature(*feature);
    }

    requirements
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, ContentId, GameContent, GamePart, Platform, TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::policy::{
        default::DefaultConflictPolicy, default::DefaultFormattingPolicy,
        default::DefaultNamingPolicy, PolicySet,
    };

    fn test_policy() -> PolicySet {
        PolicySet::new(
            Box::new(DefaultNamingPolicy),
            Box::new(DefaultFormattingPolicy),
            Box::new(DefaultConflictPolicy),
        )
    }

    #[test]
    fn compiles_single_disc_game_to_flat_file() {
        let spec = PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGames,
                NamingSpec::GameTitle,
                ArtifactSpec::new(crate::output::capabilities::ContentType::Disc)
                    .with_format(Format::Iso),
            )],
        );

        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ps2:shadow-of-the-colossus"),
            source: SourceRef::new("file:/roms/Shadow of the Colossus.chd"),
            title: "Shadow of the Colossus".to_string(),
            platform: Platform::Unknown,
            parts: vec![GamePart::Disc(crate::core::content::DiscPart {
                source: SourceRef::new("file:/roms/Shadow of the Colossus.chd"),
                disc_number: 1,
                consumed_sources: vec![],
            })],
            consumed_sources: vec![],
        })];

        let plan = compile_presentation_spec(&spec, &content, &test_policy());

        assert_eq!(plan.entries.len(), 1);

        let PlanEntry::File(file) = &plan.entries[0] else {
            panic!("expected file entry");
        };

        assert_eq!(file.name, "Shadow of the Colossus.iso");
        assert_eq!(
            file.artifact.requirements.content_type,
            crate::output::capabilities::ContentType::Disc
        );
        assert_eq!(file.artifact.requirements.format, Some(Format::Iso));
    }

    #[test]
    fn compiles_bytes_content_using_source_name() {
        let spec = PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::Bytes,
                NamingSpec::SourceName,
                ArtifactSpec::new(crate::output::capabilities::ContentType::Bytes)
                    .with_format(Format::Bin),
            )],
        );

        let content = vec![NormalizedContent::Bytes(BytesContent {
            id: ContentId::new("bytes:readme"),
            source: SourceRef::new("file:/roms/manual"),
            size: 123,
        })];

        let plan = compile_presentation_spec(&spec, &content, &test_policy());

        assert_eq!(plan.entries.len(), 1);

        let PlanEntry::File(file) = &plan.entries[0] else {
            panic!("expected file entry");
        };

        assert_eq!(file.name, "manual.bin");
        assert_eq!(file.artifact.requirements.format, Some(Format::Bin));
    }

    #[test]
    fn compiles_text_content_using_source_name() {
        let spec = PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::Text,
                NamingSpec::SourceName,
                ArtifactSpec::new(crate::output::capabilities::ContentType::Text)
                    .with_format(Format::Text),
            )],
        );

        let content = vec![NormalizedContent::Text(TextContent {
            id: ContentId::new("text:notes"),
            source: SourceRef::new("file:/roms/readme"),
            size: 42,
        })];

        let plan = compile_presentation_spec(&spec, &content, &test_policy());

        assert_eq!(plan.entries.len(), 1);

        let PlanEntry::File(file) = &plan.entries[0] else {
            panic!("expected file entry");
        };

        assert_eq!(file.name, "readme.txt");
        assert_eq!(file.artifact.requirements.format, Some(Format::Text));
    }
}
