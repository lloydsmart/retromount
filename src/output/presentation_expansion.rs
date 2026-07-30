use crate::core::content::{DiscPart, GameContent, GamePart};

/// Returns true when a game consists entirely of multiple disc parts.
pub fn is_multi_disc_game(game: &GameContent) -> bool {
    game.parts.len() > 1
        && game
            .parts
            .iter()
            .all(|part| matches!(part, GamePart::Disc(_)))
}

/// Returns disc parts sorted by disc number.
pub fn sorted_disc_parts(game: &GameContent) -> Vec<&DiscPart> {
    let mut parts: Vec<&DiscPart> = game
        .parts
        .iter()
        .filter_map(|part| match part {
            GamePart::Disc(disc) => Some(disc),
            _ => None,
        })
        .collect();

    parts.sort_by_key(|a| a.disc_number);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::ContentId;
    use crate::core::content::{DiscPart, GameContent, GamePart, Platform, RomPart};
    use crate::core::source::SourceRef;

    fn disc_part(number: u32) -> GamePart {
        GamePart::Disc(DiscPart {
            source: SourceRef::new(format!("file:/roms/disc{number}.cue")),
            disc_number: number,
            consumed_sources: vec![SourceRef::new(format!("file:/roms/disc{number}.bin"))],
            logical_disc: None,
        })
    }

    fn rom_part() -> GamePart {
        GamePart::Rom(RomPart {
            source: SourceRef::new("file:/roms/game.bin"),
            size: 2048,
        })
    }

    #[test]
    fn identifies_multi_disc_games() {
        let game = GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("file:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![disc_part(2), disc_part(1)],
            consumed_sources: vec![],
        };

        assert!(is_multi_disc_game(&game));
    }

    #[test]
    fn rejects_mixed_part_games_as_multi_disc() {
        let game = GameContent {
            id: ContentId::new("mixed"),
            source: SourceRef::new("file:/roms/mixed.bin"),
            title: "Example".to_string(),
            platform: Platform::Unknown,
            parts: vec![disc_part(1), rom_part()],
            consumed_sources: vec![],
        };

        assert!(!is_multi_disc_game(&game));
    }

    #[test]
    fn sorts_disc_parts_by_disc_number() {
        let game = GameContent {
            id: ContentId::new("example"),
            source: SourceRef::new("file:/roms/example-disc1.cue"),
            title: "Example".to_string(),
            platform: Platform::Ps1,
            parts: vec![disc_part(3), disc_part(1), disc_part(2)],
            consumed_sources: vec![],
        };

        let sorted = sorted_disc_parts(&game);
        let disc_numbers: Vec<u32> = sorted.iter().map(|disc| disc.disc_number).collect();

        assert_eq!(disc_numbers, vec![1, 2, 3]);
    }
}
