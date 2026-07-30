use serde::Serialize;
use std::fmt;
use std::sync::Arc;

use crate::core::reader_handle::ReaderHandle;
use crate::core::source::SourceRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DiscMedia {
    Cd,
    Dvd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogicalDisc {
    pub media: DiscMedia,
    pub sector_size: u32,
    pub sector_count: u64,
    pub content: ReaderHandle,
}

impl LogicalDisc {
    pub fn byte_len(&self) -> Option<u64> {
        u64::from(self.sector_size).checked_mul(self.sector_count)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DecodedContentKind {
    Bytes,
    Rom,
    Disc,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum NormalizedContentKind {
    Bytes,
    Game,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Platform {
    Snes,
    Ps1,
    Ps2,
    Nes,
    Megadrive,
    Unknown,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Snes => "snes",
            Self::Ps1 => "ps1",
            Self::Ps2 => "ps2",
            Self::Nes => "nes",
            Self::Megadrive => "megadrive",
            Self::Unknown => "unknown",
        };

        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContentId(pub Arc<str>);

impl ContentId {
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub trait ContentMeta {
    fn id(&self) -> &ContentId;
    fn source(&self) -> &SourceRef;
    fn consumed_sources(&self) -> &[SourceRef];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DecodedContent {
    Bytes(BytesContent),
    Rom(DecodedRomContent),
    Disc(DecodedDiscContent),
    Text(TextContent),
}

impl DecodedContent {
    pub fn kind(&self) -> DecodedContentKind {
        match self {
            Self::Bytes(_) => DecodedContentKind::Bytes,
            Self::Rom(_) => DecodedContentKind::Rom,
            Self::Disc(_) => DecodedContentKind::Disc,
            Self::Text(_) => DecodedContentKind::Text,
        }
    }
}

impl ContentMeta for DecodedContent {
    fn id(&self) -> &ContentId {
        match self {
            Self::Bytes(v) => &v.id,
            Self::Rom(v) => &v.id,
            Self::Disc(v) => &v.id,
            Self::Text(v) => &v.id,
        }
    }

    fn source(&self) -> &SourceRef {
        match self {
            Self::Bytes(v) => &v.source,
            Self::Rom(v) => &v.source,
            Self::Disc(v) => &v.source,
            Self::Text(v) => &v.source,
        }
    }

    fn consumed_sources(&self) -> &[SourceRef] {
        match self {
            Self::Disc(v) => &v.consumed_sources,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum NormalizedContent {
    Bytes(BytesContent),
    Game(GameContent),
    Text(TextContent),
}

impl NormalizedContent {
    pub fn kind(&self) -> NormalizedContentKind {
        match self {
            Self::Bytes(_) => NormalizedContentKind::Bytes,
            Self::Game(_) => NormalizedContentKind::Game,
            Self::Text(_) => NormalizedContentKind::Text,
        }
    }
}

impl ContentMeta for NormalizedContent {
    fn id(&self) -> &ContentId {
        match self {
            Self::Bytes(v) => &v.id,
            Self::Game(v) => &v.id,
            Self::Text(v) => &v.id,
        }
    }

    fn source(&self) -> &SourceRef {
        match self {
            Self::Bytes(v) => &v.source,
            Self::Game(v) => &v.source,
            Self::Text(v) => &v.source,
        }
    }

    fn consumed_sources(&self) -> &[SourceRef] {
        match self {
            Self::Game(v) => &v.consumed_sources,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BytesContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecodedRomContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecodedDiscContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub title: String,
    pub disc_number: u32,
    pub consumed_sources: Vec<SourceRef>,
    pub logical_disc: Option<LogicalDisc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub title: String,
    pub platform: Platform,
    pub parts: Vec<GamePart>,
    pub consumed_sources: Vec<SourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GamePart {
    Rom(RomPart),
    Disc(DiscPart),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RomPart {
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscPart {
    pub source: SourceRef,
    pub disc_number: u32,
    pub consumed_sources: Vec<SourceRef>,
    pub logical_disc: Option<LogicalDisc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::SourceRef;

    #[test]
    fn returns_decoded_content_kind() {
        let content = DecodedContent::Rom(DecodedRomContent {
            id: ContentId::new("game"),
            source: SourceRef::new("src:game"),
            size: 123,
        });

        assert_eq!(content.kind(), DecodedContentKind::Rom);
        assert_eq!(content.id().to_string(), "game");
    }

    #[test]
    fn returns_normalized_content_kind() {
        let content = NormalizedContent::Game(GameContent {
            id: ContentId::new("game"),
            source: SourceRef::new("file:/roms/game.sfc"),
            title: "game".to_string(),
            platform: Platform::Snes,
            parts: vec![GamePart::Rom(RomPart {
                source: SourceRef::new("file:/roms/game.sfc"),
                size: 123,
            })],
            consumed_sources: vec![],
        });

        assert_eq!(content.kind(), NormalizedContentKind::Game);
        assert_eq!(content.id().to_string(), "game");
        assert_eq!(content.source().to_string(), "file:/roms/game.sfc");
    }

    #[test]
    fn formats_platform_as_expected() {
        assert_eq!(Platform::Snes.to_string(), "snes");
        assert_eq!(Platform::Ps1.to_string(), "ps1");
        assert_eq!(Platform::Ps2.to_string(), "ps2");
        assert_eq!(Platform::Nes.to_string(), "nes");
        assert_eq!(Platform::Megadrive.to_string(), "megadrive");
        assert_eq!(Platform::Unknown.to_string(), "unknown");
    }

    #[test]
    fn returns_decoded_content_metadata_via_trait() {
        let content = DecodedContent::Disc(DecodedDiscContent {
            id: ContentId::new("disc-1"),
            source: SourceRef::new("cue:/roms/game-disc1.cue"),
            title: "Game".to_string(),
            disc_number: 1,
            consumed_sources: vec![SourceRef::new("cue:/roms/game-disc1.bin")],
            logical_disc: None,
        });

        assert_eq!(content.id().to_string(), "disc-1");
        assert_eq!(content.source().to_string(), "cue:/roms/game-disc1.cue");
        assert_eq!(content.consumed_sources().len(), 1);
    }

    #[test]
    fn returns_normalized_content_metadata_via_trait() {
        let content = NormalizedContent::Game(GameContent {
            id: ContentId::new("game"),
            source: SourceRef::new("cue:/roms/game-disc1.cue"),
            title: "Game".to_string(),
            platform: Platform::Ps1,
            parts: vec![GamePart::Disc(DiscPart {
                source: SourceRef::new("cue:/roms/game-disc1.cue"),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/game-disc1.bin")],
                logical_disc: None,
            })],
            consumed_sources: vec![SourceRef::new("cue:/roms/game-disc1.bin")],
        });

        assert_eq!(content.id().to_string(), "game");
        assert_eq!(content.source().to_string(), "cue:/roms/game-disc1.cue");
        assert_eq!(content.consumed_sources().len(), 1);
    }
}
