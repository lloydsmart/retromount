use crate::core::content::{GameContent, GamePart, Platform};

pub mod default;

pub trait NamingPolicy: Send + Sync {
    fn game_name(&self, game: &GameContent) -> String;
    fn part_name(&self, game: &GameContent, part: &GamePart) -> String;
    fn playlist_name(&self, game: &GameContent) -> String;
    fn platform_name(&self, platform: &Platform) -> String;
}

pub trait FormattingPolicy: Send + Sync {
    fn format_name(&self, raw: &str) -> String;
}

pub trait ConflictPolicy: Send + Sync {
    fn resolve_name_conflict(&self, proposed: &str, existing: &[String]) -> String;
}

pub struct PolicySet {
    naming: Box<dyn NamingPolicy>,
    formatting: Box<dyn FormattingPolicy>,
    conflict: Box<dyn ConflictPolicy>,
}

impl PolicySet {
    pub fn new(
        naming: Box<dyn NamingPolicy>,
        formatting: Box<dyn FormattingPolicy>,
        conflict: Box<dyn ConflictPolicy>,
    ) -> Self {
        Self {
            naming,
            formatting,
            conflict,
        }
    }

    pub fn naming(&self) -> &dyn NamingPolicy {
        self.naming.as_ref()
    }

    pub fn formatting(&self) -> &dyn FormattingPolicy {
        self.formatting.as_ref()
    }

    pub fn conflict(&self) -> &dyn ConflictPolicy {
        self.conflict.as_ref()
    }

    pub fn format_name(&self, raw: &str) -> String {
        self.formatting.format_name(raw)
    }

    pub fn resolve_name_conflict(&self, proposed: &str, existing: &[String]) -> String {
        self.conflict.resolve_name_conflict(proposed, existing)
    }
}
