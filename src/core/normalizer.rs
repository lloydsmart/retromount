use std::collections::{HashMap, HashSet};

use crate::core::content::{
    Content, ContentId, DiscContent, DiscPart, GameContent, GamePart, Platform, RomPart,
};
use crate::core::source::SourceRef;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DiscGroupKey {
    parent: String,
    title: String,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizationOptions {
    pub platform_hint: Option<Platform>,
}

#[derive(Debug, Clone)]
enum PendingOutput {
    Direct(Content),
    DiscGroup(DiscGroupKey),
}

pub fn normalize_content(contents: Vec<Content>, options: &NormalizationOptions) -> Vec<Content> {
    let consumed_by_discs = consumed_rom_sources(&contents);

    let mut pending = Vec::new();
    let mut disc_groups: HashMap<DiscGroupKey, Vec<DiscContent>> = HashMap::new();

    for content in contents {
        match content {
            Content::Rom(rom) => {
                if consumed_by_discs.contains(&rom.source) {
                    continue;
                }

                pending.push(PendingOutput::Direct(Content::Game(GameContent {
                    id: rom.id.clone(),
                    source: rom.source.clone(),
                    title: leaf_stem_from_source(&rom.source),
                    platform: options
                        .platform_hint
                        .clone()
                        .unwrap_or_else(|| derive_platform_from_source(&rom.source)),
                    parts: vec![GamePart::Rom(RomPart {
                        source: rom.source,
                        size: rom.size,
                    })],
                    consumed_sources: vec![],
                })));
            }
            Content::Disc(disc) => {
                let key = disc_group_key(&disc);

                if !disc_groups.contains_key(&key) {
                    pending.push(PendingOutput::DiscGroup(key.clone()));
                }

                disc_groups.entry(key).or_default().push(disc);
            }
            other => pending.push(PendingOutput::Direct(other)),
        }
    }

    let mut normalized = Vec::new();

    for item in pending {
        match item {
            PendingOutput::Direct(content) => normalized.push(content),
            PendingOutput::DiscGroup(key) => {
                if let Some(discs) = disc_groups.remove(&key) {
                    normalized.push(Content::Game(normalize_disc_group(&key, discs, options)));
                }
            }
        }
    }

    normalized
}

fn consumed_rom_sources(contents: &[Content]) -> HashSet<SourceRef> {
    contents
        .iter()
        .filter_map(|content| match content {
            Content::Disc(disc) => Some(disc),
            _ => None,
        })
        .flat_map(|disc| disc.consumed_sources.iter().cloned())
        .collect()
}

fn derive_platform(path: &str) -> Platform {
    let normalized = path.to_lowercase().replace('\\', "/");

    platform_from_segments(&normalized).unwrap_or_else(|| platform_from_extension(&normalized))
}

fn derive_platform_from_source(source: &SourceRef) -> Platform {
    let normalized = source_path_for_matching(source);
    derive_platform(&normalized)
}

fn source_path_for_matching(source: &SourceRef) -> String {
    let raw = source.0.as_ref();

    match raw.split_once('#') {
        Some((_, member)) => member.replace('\\', "/"),
        None => raw.replace('\\', "/"),
    }
}

fn leaf_stem_from_source(source: &SourceRef) -> String {
    let normalized = source_path_for_matching(source);
    leaf_stem(&normalized)
}

fn platform_from_segments(path: &str) -> Option<Platform> {
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        let platform = if matches_platform_segment(segment, &["snes"]) {
            Platform::Snes
        } else if matches_platform_segment(segment, &["ps1", "psx", "playstation"]) {
            Platform::Ps1
        } else if matches_platform_segment(segment, &["nes"]) {
            Platform::Nes
        } else if matches_platform_segment(segment, &["megadrive", "genesis"]) {
            Platform::Megadrive
        } else {
            continue;
        };

        return Some(platform);
    }

    None
}

fn matches_platform_segment(segment: &str, aliases: &[&str]) -> bool {
    aliases.iter().any(|alias| {
        segment == *alias
            || segment
                .strip_prefix(alias)
                .is_some_and(|rest| rest.starts_with('_') || rest.starts_with('-'))
    })
}

fn platform_from_extension(path: &str) -> Platform {
    if path.ends_with(".sfc") || path.ends_with(".smc") {
        Platform::Snes
    } else if path.ends_with(".nes") {
        Platform::Nes
    } else {
        Platform::Unknown
    }
}

fn normalize_disc_group(
    key: &DiscGroupKey,
    mut discs: Vec<DiscContent>,
    options: &NormalizationOptions,
) -> GameContent {
    discs.sort_by(|left, right| {
        left.disc_number
            .cmp(&right.disc_number)
            .then_with(|| left.id.0.cmp(&right.id.0))
    });

    let primary = discs
        .first()
        .expect("normalize_disc_group called with empty disc list");

    let parts = discs
        .iter()
        .map(|disc| {
            GamePart::Disc(DiscPart {
                source: disc.source.clone(),
                disc_number: disc.disc_number,
                consumed_sources: disc.consumed_sources.clone(),
            })
        })
        .collect();

    let consumed_sources = discs
        .iter()
        .flat_map(|disc| disc.consumed_sources.iter().cloned())
        .collect();

    GameContent {
        id: ContentId::new(game_id_for(key)),
        source: primary.source.clone(),
        title: primary.title.clone(),
        platform: options
            .platform_hint
            .clone()
            .unwrap_or_else(|| derive_platform_from_source(&primary.source)),
        parts,
        consumed_sources,
    }
}

fn disc_group_key(content: &DiscContent) -> DiscGroupKey {
    DiscGroupKey {
        parent: logical_parent_path(content.id.0.as_ref()),
        title: content.title.clone(),
    }
}

fn logical_parent_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    match normalized.rsplit_once('/') {
        Some((parent, _)) => parent.to_string(),
        None => String::new(),
    }
}

fn game_id_for(key: &DiscGroupKey) -> String {
    if key.parent.is_empty() {
        key.title.clone()
    } else {
        format!("{}/{}", key.parent, key.title)
    }
}

fn leaf_stem(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or(&normalized);

    match leaf.rsplit_once('.') {
        Some((base, _)) => base.to_string(),
        None => leaf.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{Content, ContentId, DiscContent, GamePart, RomContent};
    use crate::core::source::SourceRef;

    #[test]
    fn normalizes_rom_to_game() {
        let normalized = normalize_content(
            vec![Content::Rom(RomContent {
                id: ContentId::new("smw"),
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                size: 4096,
            })],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 1);

        let game = match &normalized[0] {
            Content::Game(game) => game,
            other => panic!("expected game, got {other:?}"),
        };

        assert_eq!(game.title, "Super Mario World");
        assert_eq!(game.platform, Platform::Snes);
        assert_eq!(game.parts.len(), 1);

        match &game.parts[0] {
            GamePart::Rom(part) => {
                assert_eq!(part.size, 4096);
            }
            other => panic!("expected rom part, got {other:?}"),
        }
    }

    #[test]
    fn groups_discs_into_single_game() {
        let normalized = normalize_content(
            vec![
                Content::Disc(DiscContent {
                    id: ContentId::new("ps1/ff7-disc2"),
                    source: SourceRef::new("cue:/roms/ps1/ff7-disc2.cue"),
                    title: "Final Fantasy VII".to_string(),
                    disc_number: 2,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ps1/ff7-disc2.bin")],
                }),
                Content::Disc(DiscContent {
                    id: ContentId::new("ps1/ff7-disc1"),
                    source: SourceRef::new("cue:/roms/ps1/ff7-disc1.cue"),
                    title: "Final Fantasy VII".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ps1/ff7-disc1.bin")],
                }),
            ],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 1);

        let game = match &normalized[0] {
            Content::Game(game) => game,
            other => panic!("expected game, got {other:?}"),
        };

        assert_eq!(game.title, "Final Fantasy VII");
        assert_eq!(game.platform, Platform::Ps1);
        assert_eq!(game.parts.len(), 2);

        match &game.parts[0] {
            GamePart::Disc(part) => assert_eq!(part.disc_number, 1),
            other => panic!("expected disc part, got {other:?}"),
        }

        match &game.parts[1] {
            GamePart::Disc(part) => assert_eq!(part.disc_number, 2),
            other => panic!("expected disc part, got {other:?}"),
        }
    }

    #[test]
    fn suppresses_roms_consumed_by_discs() {
        let normalized = normalize_content(
            vec![
                Content::Rom(RomContent {
                    id: ContentId::new("discs/ps1_multi/game_disc1.bin"),
                    source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.bin"),
                    size: 6,
                }),
                Content::Disc(DiscContent {
                    id: ContentId::new("discs/ps1_multi/game_disc1.cue"),
                    source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.cue"),
                    title: "game".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new(
                        "file:/roms/discs/ps1_multi/game_disc1.bin",
                    )],
                }),
            ],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 1);

        let game = match &normalized[0] {
            Content::Game(game) => game,
            other => panic!("expected game, got {other:?}"),
        };

        assert_eq!(game.title, "game");
        assert_eq!(game.parts.len(), 1);

        match &game.parts[0] {
            GamePart::Disc(part) => assert_eq!(part.disc_number, 1),
            other => panic!("expected disc part, got {other:?}"),
        }
    }

    #[test]
    fn does_not_merge_same_title_from_different_directories() {
        let normalized = normalize_content(
            vec![
                Content::Disc(DiscContent {
                    id: ContentId::new("ps1_multi/game_disc1.cue"),
                    source: SourceRef::new("file:/roms/ps1_multi/game_disc1.cue"),
                    title: "game".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("file:/roms/ps1_multi/game_disc1.bin")],
                }),
                Content::Disc(DiscContent {
                    id: ContentId::new("ps1_single/game.cue"),
                    source: SourceRef::new("file:/roms/ps1_single/game.cue"),
                    title: "game".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("file:/roms/ps1_single/game.bin")],
                }),
            ],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 2);
    }

    #[test]
    fn derives_rom_game_title_from_source_leaf_name() {
        let normalized = normalize_content(
            vec![Content::Rom(RomContent {
                id: ContentId::new("roms/snes/Super Mario World.sfc"),
                source: SourceRef::new("file:/roms/snes/Super Mario World.sfc"),
                size: 4096,
            })],
            &NormalizationOptions::default(),
        );

        let game = match &normalized[0] {
            Content::Game(game) => game,
            other => panic!("expected game, got {other:?}"),
        };

        assert_eq!(game.title, "Super Mario World");
    }

    #[test]
    fn derives_platform_from_path() {
        assert_eq!(derive_platform("roms/snes/game.sfc"), Platform::Snes);
        assert_eq!(derive_platform("roms/ps1/game.cue"), Platform::Ps1);
        assert_eq!(derive_platform("roms/psx/game.cue"), Platform::Ps1);
        assert_eq!(derive_platform("roms/playstation/game.cue"), Platform::Ps1);
        assert_eq!(derive_platform("roms/nes/game.nes"), Platform::Nes);
        assert_eq!(
            derive_platform("roms/megadrive/game.bin"),
            Platform::Megadrive
        );
        assert_eq!(
            derive_platform("roms/genesis/game.bin"),
            Platform::Megadrive
        );
        assert_eq!(derive_platform("roms/unknown/game.bin"), Platform::Unknown);
        assert_eq!(derive_platform("roms/game.sfc"), Platform::Snes);
        assert_eq!(derive_platform("roms/game.smc"), Platform::Snes);
        assert_eq!(derive_platform("roms/game.nes"), Platform::Nes);
        assert_eq!(derive_platform("roms/game.cue"), Platform::Unknown);
    }

    #[test]
    fn derives_platform_from_zip_member_source() {
        let source =
            SourceRef::new("zip:/roms/collection.zip#roms/megadrive/Sonic the Hedgehog.bin");
        assert_eq!(derive_platform_from_source(&source), Platform::Megadrive);
    }
}
