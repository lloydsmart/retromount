use std::collections::{HashMap, HashSet};

use crate::core::content::{
    ContentId, DecodedContent, DecodedDiscContent, DiscPart, GameContent, GamePart,
    NormalizedContent, Platform, RomPart,
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
    Direct(NormalizedContent),
    DiscGroup(DiscGroupKey),
}

/// Transforms decoded content into normalized domain models.
///
/// This is the canonical boundary between decoding and presentation.
///
/// - Input: `DecodedContent`
/// - Output: `NormalizedContent`
///
/// All grouping, classification, and semantic enrichment happens here.
/// Downstream stages must not depend on decoder-specific details.
pub(crate) fn normalize_decoded_content(
    contents: Vec<DecodedContent>,
    options: &NormalizationOptions,
) -> Vec<NormalizedContent> {
    let consumed_by_discs = consumed_rom_sources(&contents);

    let mut pending = Vec::new();
    let mut disc_groups: HashMap<DiscGroupKey, Vec<DecodedDiscContent>> = HashMap::new();

    for content in contents {
        match content {
            DecodedContent::Rom(rom) => {
                if consumed_by_discs.contains(&rom.source) {
                    continue;
                }

                pending.push(PendingOutput::Direct(NormalizedContent::Game(
                    GameContent {
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
                    },
                )));
            }
            DecodedContent::Disc(disc) => {
                let key = disc_group_key(&disc);

                if !disc_groups.contains_key(&key) {
                    pending.push(PendingOutput::DiscGroup(key.clone()));
                }

                disc_groups.entry(key).or_default().push(disc);
            }
            DecodedContent::Bytes(bytes) => {
                pending.push(PendingOutput::Direct(NormalizedContent::Bytes(bytes)));
            }
            DecodedContent::Text(text) => {
                pending.push(PendingOutput::Direct(NormalizedContent::Text(text)));
            }
        }
    }

    let mut normalized = Vec::new();

    for item in pending {
        match item {
            PendingOutput::Direct(content) => normalized.push(content),
            PendingOutput::DiscGroup(key) => {
                if let Some(discs) = disc_groups.remove(&key) {
                    normalized.push(NormalizedContent::Game(normalize_disc_group(
                        &key, discs, options,
                    )));
                }
            }
        }
    }

    normalized
}

fn consumed_rom_sources(contents: &[DecodedContent]) -> HashSet<SourceRef> {
    contents
        .iter()
        .filter_map(|content| match content {
            DecodedContent::Disc(disc) => Some(disc),
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
    mut discs: Vec<DecodedDiscContent>,
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
                logical_disc: disc.logical_disc.clone(),
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

fn disc_group_key(content: &DecodedDiscContent) -> DiscGroupKey {
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
    use crate::core::content::{
        ContentId, DecodedContent, DecodedContentKind, DecodedDiscContent, DecodedRomContent,
        DiscMedia, GamePart, LogicalDisc, NormalizedContent, NormalizedContentKind, TextContent,
    };
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::SourceRef;
    use crate::readers::inline_reader::InlineReader;

    #[test]
    fn normalizes_rom_to_game() {
        let normalized = normalize_decoded_content(
            vec![DecodedContent::Rom(DecodedRomContent {
                id: ContentId::new("smw"),
                source: SourceRef::new("file:/roms/Super Mario World.sfc"),
                size: 4096,
            })],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 1);

        let game = match &normalized[0] {
            NormalizedContent::Game(game) => game,
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
        let normalized = normalize_decoded_content(
            vec![
                DecodedContent::Disc(DecodedDiscContent {
                    id: ContentId::new("ps1/ff7-disc2"),
                    source: SourceRef::new("cue:/roms/ps1/ff7-disc2.cue"),
                    title: "Final Fantasy VII".to_string(),
                    disc_number: 2,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ps1/ff7-disc2.bin")],
                    logical_disc: None,
                }),
                DecodedContent::Disc(DecodedDiscContent {
                    id: ContentId::new("ps1/ff7-disc1"),
                    source: SourceRef::new("cue:/roms/ps1/ff7-disc1.cue"),
                    title: "Final Fantasy VII".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("cue:/roms/ps1/ff7-disc1.bin")],
                    logical_disc: None,
                }),
            ],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 1);

        let game = match &normalized[0] {
            NormalizedContent::Game(game) => game,
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
        let normalized = normalize_decoded_content(
            vec![
                DecodedContent::Rom(DecodedRomContent {
                    id: ContentId::new("discs/ps1_multi/game_disc1.bin"),
                    source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.bin"),
                    size: 6,
                }),
                DecodedContent::Disc(DecodedDiscContent {
                    id: ContentId::new("discs/ps1_multi/game_disc1.cue"),
                    source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.cue"),
                    title: "game".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new(
                        "file:/roms/discs/ps1_multi/game_disc1.bin",
                    )],
                    logical_disc: None,
                }),
            ],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 1);

        let game = match &normalized[0] {
            NormalizedContent::Game(game) => game,
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
        let normalized = normalize_decoded_content(
            vec![
                DecodedContent::Disc(DecodedDiscContent {
                    id: ContentId::new("ps1_multi/game_disc1.cue"),
                    source: SourceRef::new("file:/roms/ps1_multi/game_disc1.cue"),
                    title: "game".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("file:/roms/ps1_multi/game_disc1.bin")],
                    logical_disc: None,
                }),
                DecodedContent::Disc(DecodedDiscContent {
                    id: ContentId::new("ps1_single/game.cue"),
                    source: SourceRef::new("file:/roms/ps1_single/game.cue"),
                    title: "game".to_string(),
                    disc_number: 1,
                    consumed_sources: vec![SourceRef::new("file:/roms/ps1_single/game.bin")],
                    logical_disc: None,
                }),
            ],
            &NormalizationOptions::default(),
        );

        assert_eq!(normalized.len(), 2);
    }

    #[test]
    fn derives_rom_game_title_from_source_leaf_name() {
        let normalized = normalize_decoded_content(
            vec![DecodedContent::Rom(DecodedRomContent {
                id: ContentId::new("roms/snes/Super Mario World.sfc"),
                source: SourceRef::new("file:/roms/snes/Super Mario World.sfc"),
                size: 4096,
            })],
            &NormalizationOptions::default(),
        );

        let game = match &normalized[0] {
            NormalizedContent::Game(game) => game,
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

    #[test]
    fn returns_expected_decoded_content_kind() {
        let content = DecodedContent::Rom(DecodedRomContent {
            id: ContentId::new("sonic"),
            source: SourceRef::new("file:/roms/sonic.bin"),
            size: 1024,
        });

        assert_eq!(content.kind(), DecodedContentKind::Rom);
    }

    #[test]
    fn returns_expected_normalized_content_kind() {
        let content = NormalizedContent::Text(TextContent {
            id: ContentId::new("readme"),
            source: SourceRef::new("file:/roms/readme.txt"),
            size: 10,
        });

        assert_eq!(content.kind(), NormalizedContentKind::Text);
    }

    #[test]
    fn preserves_logical_disc_through_normalization() {
        let decoded = DecodedContent::Disc(DecodedDiscContent {
            id: ContentId::new("ps2/game.chd"),
            source: SourceRef::new("file:/roms/ps2/game.chd"),
            title: "Game".to_string(),
            disc_number: 1,
            consumed_sources: vec![],
            logical_disc: Some(LogicalDisc {
                media: DiscMedia::Dvd,
                sector_size: 2048,
                sector_count: 2,
                content: ReaderHandle::new("test:ps2-dvd", || {
                    Ok(Box::new(InlineReader::new(vec![0; 4096])))
                }),
            }),
        });

        let normalized = normalize_decoded_content(vec![decoded], &NormalizationOptions::default());
        let NormalizedContent::Game(game) = &normalized[0] else {
            panic!("expected normalized game");
        };
        let GamePart::Disc(disc) = &game.parts[0] else {
            panic!("expected normalized disc");
        };
        let logical_disc = disc.logical_disc.as_ref().expect("logical disc");

        assert_eq!(logical_disc.media, DiscMedia::Dvd);
        assert_eq!(logical_disc.byte_len(), Some(4096));
        assert_eq!(logical_disc.content.id(), "test:ps2-dvd");
    }
}
