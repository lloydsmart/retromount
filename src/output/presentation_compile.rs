use crate::core::content::{ContentMeta, GameContent, GamePart, NormalizedContent};
use crate::core::source::SourceRef;
use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
use crate::output::plan::{
    ArtifactId, ArtifactReference, ArtifactRequest, ContentArtifact, GeneratedArtifact,
    PlanArtifactSet, PlanDirectory, PlanEntry, PlanFile, PlannedArtifactKind, PlaylistArtifact,
    PresentationPlan, SourceArtifact,
};
use crate::output::presentation_expansion::{is_multi_disc_game, sorted_disc_parts};
use crate::output::presentation_spec::{
    ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
};
use crate::policy::PolicySet;

/// Compiles a declarative `PresentationSpec` into a concrete `PresentationPlan`.
///
/// This initial compiler is intentionally minimal:
///
/// - supports flat layout and a minimal grouped layout
/// - emits files and simple directories only
/// - supports single-source artifacts only
/// - supports generated playlists for multi-disc games
pub fn compile_presentation_spec(
    spec: &PresentationSpec,
    content: &[NormalizedContent],
    policy: &PolicySet,
) -> PresentationPlan {
    match &spec.layout {
        LayoutSpec::Flat => compile_flat(spec, content, policy),
        LayoutSpec::GroupedByPlatformAndGame => {
            compile_grouped_by_platform_and_game(spec, content, policy)
        }
        LayoutSpec::LiteralRoot(name) => {
            let flat = compile_flat(spec, content, policy);
            PresentationPlan::new(vec![PlanEntry::Directory(PlanDirectory::new(
                name.clone(),
                flat.entries,
            ))])
        }
    }
}

fn compile_flat(
    spec: &PresentationSpec,
    content: &[NormalizedContent],
    policy: &PolicySet,
) -> PresentationPlan {
    let mut root = PlanDirectory::new("", Vec::new());

    for item in content {
        for rule in &spec.files {
            let directory = ensure_directory(&mut root, &rule.directory, policy);
            let mut names = child_names(directory);

            directory
                .entries
                .extend(try_compile_rule(rule, item, policy, &mut names));
        }
    }

    PresentationPlan::new(root.entries)
}

fn compile_grouped_by_platform_and_game(
    spec: &PresentationSpec,
    content: &[NormalizedContent],
    policy: &PolicySet,
) -> PresentationPlan {
    let mut root = PlanDirectory::new("", Vec::new());

    for item in content {
        match item {
            NormalizedContent::Game(game) => {
                let platform_name =
                    policy.format_name(&policy.naming().platform_name(&game.platform));
                let game_dir_name = policy.format_name(&policy.naming().game_name(game));

                let platform_dir = ensure_directory(&mut root, &[platform_name], policy);
                let game_dir =
                    ensure_distinct_child_directory(platform_dir, &game_dir_name, policy);
                for rule in &spec.files {
                    let directory = ensure_directory(game_dir, &rule.directory, policy);
                    let mut names = child_names(directory);

                    directory
                        .entries
                        .extend(try_compile_rule(rule, item, policy, &mut names));
                }
            }
            NormalizedContent::Bytes(_) | NormalizedContent::Text(_) => {
                let (parents, _) = split_parent_dirs(&item.id().to_string());
                let content_directory = ensure_directory(&mut root, &parents, policy);

                for rule in &spec.files {
                    let directory = ensure_directory(content_directory, &rule.directory, policy);
                    let mut names = child_names(directory);

                    directory
                        .entries
                        .extend(try_compile_rule(rule, item, policy, &mut names));
                }
            }
        }
    }

    PresentationPlan::new(root.entries)
}

fn try_compile_rule(
    rule: &FileRuleSpec,
    item: &NormalizedContent,
    policy: &PolicySet,
    root_names: &mut Vec<String>,
) -> Vec<PlanEntry> {
    if rule.select == SelectSpec::MultiDiscGames {
        return try_compile_multi_disc_rule(rule, item, policy, root_names);
    }

    let match_result = match rule.select {
        SelectSpec::Games => match_game(item),
        SelectSpec::GamesWithoutParts => match_game_without_parts(item),
        SelectSpec::SingleDiscGames => match_single_disc_game(item),
        SelectSpec::SingleDiscGamesByPlatform { platform } => {
            match_single_disc_game_by_platform(item, platform)
        }
        SelectSpec::SingleDiscGamesByPlatformAndMedia { platform, media } => {
            match_single_disc_game_by_platform_and_media(item, platform, media)
        }
        SelectSpec::MultiDiscGames => unreachable!("handled above"),
        SelectSpec::SingleRomGames => match_single_rom_game(item),
        SelectSpec::Bytes => match_bytes(item),
        SelectSpec::Text => match_text(item),
    };

    let Some(match_result) = match_result else {
        return Vec::new();
    };

    let Some(proposed_name) = build_name(&rule.naming, item, &rule.artifact, policy) else {
        return Vec::new();
    };
    if rule.artifact.format == Some(Format::CueBin) {
        let NormalizedContent::Game(game) = item else {
            return Vec::new();
        };
        let Some(cd_disc) = match_result.cd_disc else {
            return Vec::new();
        };
        let directory_name =
            policy.resolve_name_conflict(&policy.naming().game_name(game), root_names);
        root_names.push(directory_name.clone());
        let artifact_name = strip_known_extension(&proposed_name, Some(Format::CueBin));
        let artifact = ArtifactRequest::new(
            build_artifact_id(&rule.select, item),
            PlannedArtifactKind::ContentBacked(ContentArtifact::cd_disc(cd_disc)),
            build_requirements(&rule.artifact),
        );
        return vec![PlanEntry::ArtifactSet(PlanArtifactSet::new(
            directory_name,
            artifact_name,
            artifact,
        ))];
    }

    let proposed_name = apply_extension(proposed_name, rule.artifact.format);
    let allocated_name = policy.resolve_name_conflict(&proposed_name, root_names);
    root_names.push(allocated_name.clone());

    let artifact = ArtifactRequest::new(
        build_artifact_id(&rule.select, item),
        match match_result.logical_disc {
            Some(logical_disc) => {
                PlannedArtifactKind::ContentBacked(ContentArtifact::logical_disc(logical_disc))
            }
            None => PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                match_result.source,
                match_result.size,
            )),
        },
        build_requirements(&rule.artifact),
    );

    vec![PlanEntry::File(PlanFile::new(allocated_name, artifact))]
}

fn try_compile_multi_disc_rule(
    rule: &FileRuleSpec,
    item: &NormalizedContent,
    policy: &PolicySet,
    names: &mut Vec<String>,
) -> Vec<PlanEntry> {
    let NormalizedContent::Game(game) = item else {
        return Vec::new();
    };

    if !is_multi_disc_game(game) {
        return Vec::new();
    }

    match rule.artifact.content_type {
        ContentType::Disc => sorted_disc_parts(game)
            .into_iter()
            .filter_map(|disc| {
                let proposed_name = match &rule.naming {
                    NamingSpec::PartName => policy
                        .naming()
                        .part_name(game, &GamePart::Disc(disc.clone())),
                    _ => build_name(&rule.naming, item, &rule.artifact, policy)?,
                };
                let proposed_name = apply_extension(proposed_name, rule.artifact.format);
                let allocated_name = policy.resolve_name_conflict(&proposed_name, names);
                names.push(allocated_name.clone());

                let artifact_id =
                    ArtifactId::new(format!("game:{}:disc:{}", game.id, disc.disc_number));
                Some(PlanEntry::File(PlanFile::new(
                    allocated_name,
                    ArtifactRequest::new(
                        artifact_id,
                        match &disc.logical_disc {
                            Some(logical_disc) => PlannedArtifactKind::ContentBacked(
                                ContentArtifact::logical_disc(logical_disc.clone()),
                            ),
                            None => PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                                disc.source.clone(),
                                0,
                            )),
                        },
                        build_requirements(&rule.artifact),
                    ),
                )))
            })
            .collect(),
        ContentType::Playlist => {
            let Some(proposed_name) = build_name(&rule.naming, item, &rule.artifact, policy) else {
                return Vec::new();
            };
            let proposed_name = apply_extension(proposed_name, rule.artifact.format);
            let allocated_name = policy.resolve_name_conflict(&proposed_name, names);
            names.push(allocated_name.clone());

            let references = sorted_disc_parts(game)
                .into_iter()
                .map(|disc| {
                    ArtifactReference::new(ArtifactId::new(format!(
                        "game:{}:disc:{}",
                        game.id, disc.disc_number
                    )))
                })
                .collect();

            vec![PlanEntry::File(PlanFile::new(
                allocated_name,
                ArtifactRequest::new(
                    ArtifactId::new(format!("game:{}:playlist", game.id)),
                    PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(
                        PlaylistArtifact::new(references),
                    )),
                    build_requirements(&rule.artifact),
                ),
            ))]
        }
        _ => Vec::new(),
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
                .map(str::to_string)
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
                let resolved_name = policy.resolve_name_conflict(segment, &child_names(current));
                current
                    .entries
                    .push(PlanEntry::Directory(PlanDirectory::new(
                        &resolved_name,
                        Vec::new(),
                    )));
                current
                    .entries
                    .iter()
                    .position(
                        |entry| matches!(entry, PlanEntry::Directory(dir) if dir.name == resolved_name),
                    )
                    .expect("inserted directory should be present")
            }
        };

        current = match current.entries.get_mut(index) {
            Some(PlanEntry::Directory(directory)) => directory,
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
    let resolved_name = policy.resolve_name_conflict(proposed, &child_names(parent));
    parent.entries.push(PlanEntry::Directory(PlanDirectory::new(
        &resolved_name,
        Vec::new(),
    )));

    match parent.entries.last_mut() {
        Some(PlanEntry::Directory(directory)) => directory,
        _ => unreachable!("inserted child should be a directory"),
    }
}

fn child_names(directory: &PlanDirectory) -> Vec<String> {
    directory
        .entries
        .iter()
        .map(|entry| match entry {
            PlanEntry::Directory(directory) => directory.name.clone(),
            PlanEntry::File(file) => file.name.clone(),
            PlanEntry::ArtifactSet(set) => set.directory_name.clone(),
        })
        .collect()
}

struct SourceMatch {
    source: SourceRef,
    size: u64,
    logical_disc: Option<crate::core::content::LogicalDisc>,
    cd_disc: Option<crate::core::cd::CdDisc>,
}

fn match_game(item: &NormalizedContent) -> Option<SourceMatch> {
    match item {
        NormalizedContent::Game(game) => Some(SourceMatch {
            source: game.source.clone(),
            size: 0,
            logical_disc: None,
            cd_disc: None,
        }),
        _ => None,
    }
}

fn match_game_without_parts(item: &NormalizedContent) -> Option<SourceMatch> {
    let NormalizedContent::Game(game) = item else {
        return None;
    };

    game.parts.is_empty().then(|| SourceMatch {
        source: game.source.clone(),
        size: 0,
        logical_disc: None,
        cd_disc: None,
    })
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
            size: 0,
            logical_disc: disc.logical_disc.clone(),
            cd_disc: disc.cd_disc.clone(),
        }),
        GamePart::Rom(_) => None,
    }
}

fn match_single_disc_game_by_platform(
    item: &NormalizedContent,
    platform: crate::core::content::Platform,
) -> Option<SourceMatch> {
    let NormalizedContent::Game(game) = item else {
        return None;
    };
    (game.platform == platform)
        .then(|| match_single_disc_game(item))
        .flatten()
}

fn match_single_disc_game_by_platform_and_media(
    item: &NormalizedContent,
    platform: crate::core::content::Platform,
    media: crate::core::content::DiscMedia,
) -> Option<SourceMatch> {
    let NormalizedContent::Game(game) = item else {
        return None;
    };

    if game.platform != platform {
        return None;
    }

    let matched = match_single_disc_game(item)?;
    (matched.logical_disc.as_ref()?.media == media).then_some(matched)
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
            logical_disc: None,
            cd_disc: None,
        }),
        GamePart::Disc(_) => None,
    }
}

fn match_bytes(item: &NormalizedContent) -> Option<SourceMatch> {
    match item {
        NormalizedContent::Bytes(bytes) => Some(SourceMatch {
            source: bytes.source.clone(),
            size: bytes.size,
            logical_disc: None,
            cd_disc: None,
        }),
        _ => None,
    }
}

fn match_text(item: &NormalizedContent) -> Option<SourceMatch> {
    match item {
        NormalizedContent::Text(text) => Some(SourceMatch {
            source: text.source.clone(),
            size: text.size,
            logical_disc: None,
            cd_disc: None,
        }),
        _ => None,
    }
}

fn build_name(
    naming: &NamingSpec,
    item: &NormalizedContent,
    artifact: &ArtifactSpec,
    policy: &PolicySet,
) -> Option<String> {
    match naming {
        NamingSpec::GameTitle => game_title(item),
        NamingSpec::GameName => game_name(item, policy),
        NamingSpec::PartName => part_name(item, policy),
        NamingSpec::PlaylistName => playlist_name(item, policy),
        NamingSpec::SourceName => source_name(item, artifact),
        NamingSpec::Literal(value) => Some(value.clone()),
    }
}

fn game_name(item: &NormalizedContent, policy: &PolicySet) -> Option<String> {
    match item {
        NormalizedContent::Game(game) => Some(policy.naming().game_name(game)),
        _ => None,
    }
}

fn playlist_name(item: &NormalizedContent, policy: &PolicySet) -> Option<String> {
    match item {
        NormalizedContent::Game(game) => Some(policy.naming().playlist_name(game)),
        _ => None,
    }
}

fn game_title(item: &NormalizedContent) -> Option<String> {
    match item {
        NormalizedContent::Game(game) => Some(game.title.clone()),
        _ => None,
    }
}

fn part_name(item: &NormalizedContent, policy: &PolicySet) -> Option<String> {
    let NormalizedContent::Game(game) = item else {
        return None;
    };

    if game.parts.len() != 1 {
        return None;
    }

    let part = &game.parts[0];
    Some(policy.naming().part_name(game, part))
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
    if has_extension(&name) {
        return name;
    }

    match extension_for_format(format) {
        Some(ext) => format!("{name}.{ext}"),
        None => name,
    }
}

fn has_extension(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some()
}

fn extension_for_format(format: Option<Format>) -> Option<&'static str> {
    match format {
        Some(Format::Iso) => Some("iso"),
        Some(Format::Chd) => Some("chd"),
        Some(Format::Zip) => Some("zip"),
        Some(Format::M3u) => Some("m3u"),
        Some(Format::Directory) => None,
        Some(Format::Bin) => Some("bin"),
        Some(Format::CueBin) => Some("cue"),
        Some(Format::Text) => Some("txt"),
        None => None,
    }
}

fn build_artifact_id(select: &SelectSpec, item: &NormalizedContent) -> ArtifactId {
    match (select, item) {
        (SelectSpec::Games, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
        }
        (SelectSpec::GamesWithoutParts, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
        }
        (SelectSpec::SingleDiscGames, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
        }
        (SelectSpec::SingleDiscGamesByPlatformAndMedia { .. }, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
        }
        (SelectSpec::MultiDiscGames, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
        }
        (SelectSpec::SingleRomGames, NormalizedContent::Game(game)) => {
            ArtifactId::new(format!("game:{}", game.id))
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
        BytesContent, ContentId, DiscPart, GameContent, GamePart, Platform, TextContent,
    };
    use crate::policy::{
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

    #[test]
    fn compiles_single_disc_game_to_flat_file() {
        let spec = PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGames,
                NamingSpec::PartName,
                ArtifactSpec::new(crate::output::capabilities::ContentType::Disc)
                    .with_format(Format::Bin),
            )],
        );

        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ps2:shadow-of-the-colossus"),
            source: SourceRef::new("file:/roms/Shadow of the Colossus.chd"),
            title: "Shadow of the Colossus".to_string(),
            platform: Platform::Unknown,
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("file:/roms/Shadow of the Colossus.chd"),
                disc_number: 1,
                consumed_sources: vec![],
                cd_disc: None,
                logical_disc: None,
            })],
            consumed_sources: vec![],
        })];

        let plan = compile_presentation_spec(&spec, &content, &test_policy());

        assert_eq!(plan.entries.len(), 1);

        let PlanEntry::File(file) = &plan.entries[0] else {
            panic!("expected file entry");
        };

        assert_eq!(file.name, "Shadow of the Colossus (Disc 1).cue");
        assert_eq!(
            file.artifact.requirements.content_type,
            crate::output::capabilities::ContentType::Disc
        );
        assert_eq!(file.artifact.requirements.format, Some(Format::Bin));
    }

    #[test]
    fn compiles_rule_files_into_a_destination_directory() {
        let spec = PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGames,
                NamingSpec::GameTitle,
                ArtifactSpec::new(crate::output::capabilities::ContentType::Disc)
                    .with_format(Format::Iso),
            )
            .in_directory(["CD"])],
        );
        let content = vec![NormalizedContent::Game(GameContent {
            id: ContentId::new("ps2:game"),
            source: SourceRef::new("file:/roms/Game.iso"),
            title: "Game".to_string(),
            platform: Platform::Ps2,
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("file:/roms/Game.iso"),
                disc_number: 1,
                consumed_sources: vec![],
                cd_disc: None,
                logical_disc: None,
            })],
            consumed_sources: vec![],
        })];

        let plan = compile_presentation_spec(&spec, &content, &test_policy());

        let [PlanEntry::Directory(directory)] = plan.entries.as_slice() else {
            panic!("expected destination directory");
        };
        assert_eq!(directory.name, "CD");
        let [PlanEntry::File(file)] = directory.entries.as_slice() else {
            panic!("expected file in destination directory");
        };
        assert_eq!(file.name, "Game.iso");
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
