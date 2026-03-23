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

#[derive(Debug, Clone)]
enum PendingOutput {
    Direct(Content),
    DiscGroup(DiscGroupKey),
}

pub fn normalize_content(contents: Vec<Content>) -> Vec<Content> {
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
                    title: leaf_stem(&rom.file_name),
                    platform: derive_platform(&rom.file_name),
                    parts: vec![GamePart::Rom(RomPart {
                        source: rom.source,
                        file_name: rom.file_name,
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
                    normalized.push(Content::Game(normalize_disc_group(&key, discs)));
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

fn platform_from_segments(path: &str) -> Option<Platform> {
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        let platform = match segment {
            "snes" => Platform::Snes,
            "ps1" | "psx" | "playstation" => Platform::Ps1,
            "nes" => Platform::Nes,
            "megadrive" | "genesis" => Platform::Megadrive,
            _ => continue,
        };

        return Some(platform);
    }

    None
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

fn normalize_disc_group(key: &DiscGroupKey, mut discs: Vec<DiscContent>) -> GameContent {
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
        platform: derive_platform(&primary.source.to_string()),
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
        let normalized = normalize_content(vec![Content::Rom(RomContent {
            id: ContentId::new("smw"),
            source: SourceRef::new("file:/roms/Super Mario World.sfc"),
            file_name: "Super Mario World.sfc".to_string(),
            size: 4096,
        })]);

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
                assert_eq!(part.file_name, "Super Mario World.sfc");
                assert_eq!(part.size, 4096);
            }
            other => panic!("expected rom part, got {other:?}"),
        }
    }

    #[test]
    fn groups_discs_into_single_game() {
        let normalized = normalize_content(vec![
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
        ]);

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
        let normalized = normalize_content(vec![
            Content::Rom(RomContent {
                id: ContentId::new("discs/ps1_multi/game_disc1.bin"),
                source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.bin"),
                file_name: "discs/ps1_multi/game_disc1.bin".to_string(),
                size: 6,
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("discs/ps1_multi/game_disc1.cue"),
                source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.cue"),
                title: "game".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.bin")],
            }),
        ]);

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
        let normalized = normalize_content(vec![
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
        ]);

        assert_eq!(normalized.len(), 2);
    }

    #[test]
    fn derives_rom_game_title_from_leaf_name() {
        let normalized = normalize_content(vec![Content::Rom(RomContent {
            id: ContentId::new("roms/snes/Super Mario World.sfc"),
            source: SourceRef::new("file:/roms/snes/Super Mario World.sfc"),
            file_name: "roms/snes/Super Mario World.sfc".to_string(),
            size: 4096,
        })]);

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
}
