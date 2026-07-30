use crate::core::content::{GameContent, GamePart, Platform};
use crate::policy::{ConflictPolicy, FormattingPolicy, NamingPolicy, PolicySet};

pub struct DefaultNamingPolicy;
pub struct DefaultFormattingPolicy;
pub struct DefaultConflictPolicy;

impl NamingPolicy for DefaultNamingPolicy {
    fn game_name(&self, game: &GameContent) -> String {
        game.title.clone()
    }

    fn part_name(&self, game: &GameContent, part: &GamePart) -> String {
        match part {
            GamePart::Rom(rom) => rom.source.file_name(),
            GamePart::Disc(disc) => format!("{} (Disc {}).cue", game.title, disc.disc_number),
        }
    }

    fn playlist_name(&self, game: &GameContent) -> String {
        format!("{}.m3u", game.title)
    }

    fn platform_name(&self, platform: &Platform) -> String {
        platform.to_string()
    }
}

impl FormattingPolicy for DefaultFormattingPolicy {
    fn format_name(&self, raw: &str) -> String {
        raw.to_string()
    }
}

impl ConflictPolicy for DefaultConflictPolicy {
    fn resolve_name_conflict(&self, proposed: &str, _existing: &[String]) -> String {
        proposed.to_string()
    }
}

impl Default for PolicySet {
    fn default() -> Self {
        Self::new(
            Box::new(DefaultNamingPolicy),
            Box::new(DefaultFormattingPolicy),
            Box::new(DefaultConflictPolicy),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{ContentId, DiscPart, GameContent, GamePart, Platform, RomPart};
    use crate::core::source::SourceRef;

    #[test]
    fn default_naming_policy_formats_disc_parts_like_current_behavior() {
        let game = GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![],
            consumed_sources: vec![],
        };

        let part = GamePart::Disc(DiscPart {
            source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
            disc_number: 2,
            consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc2.bin")],
            cd_disc: None,
            logical_disc: None,
        });

        let policy = DefaultNamingPolicy;
        assert_eq!(
            policy.part_name(&game, &part),
            "Final Fantasy VII (Disc 2).cue"
        );
    }

    #[test]
    fn default_naming_policy_uses_source_file_name_for_rom_parts() {
        let game = GameContent {
            id: ContentId::new("sonic"),
            source: SourceRef::new("zip:/roms/megadrive.zip#Sonic the Hedgehog.bin"),
            title: "Sonic the Hedgehog".to_string(),
            platform: Platform::Megadrive,
            parts: vec![],
            consumed_sources: vec![],
        };

        let part = GamePart::Rom(RomPart {
            source: SourceRef::new("zip:/roms/megadrive.zip#Sonic the Hedgehog.bin"),
            size: 1024,
        });

        let policy = DefaultNamingPolicy;
        assert_eq!(policy.part_name(&game, &part), "Sonic the Hedgehog.bin");
    }

    #[test]
    fn default_naming_policy_formats_playlist_like_current_behavior() {
        let game = GameContent {
            id: ContentId::new("ff7"),
            source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::Ps1,
            parts: vec![],
            consumed_sources: vec![],
        };

        let policy = DefaultNamingPolicy;
        assert_eq!(policy.playlist_name(&game), "Final Fantasy VII.m3u");
    }

    #[test]
    fn default_formatting_policy_is_passthrough() {
        let policy = DefaultFormattingPolicy;
        assert_eq!(
            policy.format_name("Metal Gear Solid (Disc 1).cue"),
            "Metal Gear Solid (Disc 1).cue"
        );
    }

    #[test]
    fn default_conflict_policy_preserves_proposed_name() {
        let policy = DefaultConflictPolicy;
        assert_eq!(
            policy.resolve_name_conflict("game.cue", &["other.cue".to_string()]),
            "game.cue"
        );
    }
}
