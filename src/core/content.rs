use serde::Serialize;
use std::fmt;
use std::sync::Arc;

use crate::core::source::SourceRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ContentKind {
    Bytes,
    Rom,
    Disc,
    Game,
    Text,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Platform {
    Snes,
    Ps1,
    Nes,
    Megadrive,
    Unknown,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Snes => "snes",
            Self::Ps1 => "ps1",
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

/// Transitional model retained while D-005 is implemented incrementally.
///
/// This will be removed once decode/normalize/present stages are fully migrated
/// onto `DecodedContent` and `NormalizedContent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Content {
    Bytes(BytesContent),
    Rom(RomContent),
    Disc(DiscContent),
    Game(GameContent),
    Text(TextContent),
}

impl Content {
    pub fn kind(&self) -> ContentKind {
        match self {
            Self::Bytes(_) => ContentKind::Bytes,
            Self::Rom(_) => ContentKind::Rom,
            Self::Disc(_) => ContentKind::Disc,
            Self::Game(_) => ContentKind::Game,
            Self::Text(_) => ContentKind::Text,
        }
    }

    pub fn id(&self) -> &ContentId {
        match self {
            Self::Bytes(v) => &v.id,
            Self::Rom(v) => &v.id,
            Self::Disc(v) => &v.id,
            Self::Game(v) => &v.id,
            Self::Text(v) => &v.id,
        }
    }

    pub fn source(&self) -> &SourceRef {
        match self {
            Self::Bytes(v) => &v.source,
            Self::Rom(v) => &v.source,
            Self::Disc(v) => &v.source,
            Self::Game(v) => &v.source,
            Self::Text(v) => &v.source,
        }
    }

    pub fn consumed_sources(&self) -> &[SourceRef] {
        match self {
            Self::Disc(v) => &v.consumed_sources,
            Self::Game(v) => &v.consumed_sources,
            _ => &[],
        }
    }
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

    pub fn id(&self) -> &ContentId {
        match self {
            Self::Bytes(v) => &v.id,
            Self::Rom(v) => &v.id,
            Self::Disc(v) => &v.id,
            Self::Text(v) => &v.id,
        }
    }

    pub fn source(&self) -> &SourceRef {
        match self {
            Self::Bytes(v) => &v.source,
            Self::Rom(v) => &v.source,
            Self::Disc(v) => &v.source,
            Self::Text(v) => &v.source,
        }
    }

    pub fn consumed_sources(&self) -> &[SourceRef] {
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

    pub fn id(&self) -> &ContentId {
        match self {
            Self::Bytes(v) => &v.id,
            Self::Game(v) => &v.id,
            Self::Text(v) => &v.id,
        }
    }

    pub fn source(&self) -> &SourceRef {
        match self {
            Self::Bytes(v) => &v.source,
            Self::Game(v) => &v.source,
            Self::Text(v) => &v.source,
        }
    }

    pub fn consumed_sources(&self) -> &[SourceRef] {
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
pub struct RomContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub title: String,
    pub disc_number: u32,
    pub consumed_sources: Vec<SourceRef>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

impl From<RomContent> for DecodedRomContent {
    fn from(value: RomContent) -> Self {
        Self {
            id: value.id,
            source: value.source,
            size: value.size,
        }
    }
}

impl From<DiscContent> for DecodedDiscContent {
    fn from(value: DiscContent) -> Self {
        Self {
            id: value.id,
            source: value.source,
            title: value.title,
            disc_number: value.disc_number,
            consumed_sources: value.consumed_sources,
        }
    }
}

impl From<DecodedRomContent> for RomContent {
    fn from(value: DecodedRomContent) -> Self {
        Self {
            id: value.id,
            source: value.source,
            size: value.size,
        }
    }
}

impl From<DecodedDiscContent> for DiscContent {
    fn from(value: DecodedDiscContent) -> Self {
        Self {
            id: value.id,
            source: value.source,
            title: value.title,
            disc_number: value.disc_number,
            consumed_sources: value.consumed_sources,
        }
    }
}

impl From<Content> for DecodedContent {
    fn from(value: Content) -> Self {
        match value {
            Content::Bytes(v) => Self::Bytes(v),
            Content::Rom(v) => Self::Rom(v.into()),
            Content::Disc(v) => Self::Disc(v.into()),
            Content::Text(v) => Self::Text(v),
            Content::Game(_) => {
                panic!("cannot convert normalized game content into DecodedContent")
            }
        }
    }
}

impl TryFrom<Content> for NormalizedContent {
    type Error = &'static str;

    fn try_from(value: Content) -> Result<Self, Self::Error> {
        match value {
            Content::Bytes(v) => Ok(Self::Bytes(v)),
            Content::Game(v) => Ok(Self::Game(v)),
            Content::Text(v) => Ok(Self::Text(v)),
            Content::Rom(_) | Content::Disc(_) => {
                Err("pre-normalized content cannot be converted into NormalizedContent")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::SourceRef;

    #[test]
    fn returns_content_kind() {
        let content = Content::Rom(RomContent {
            id: ContentId::new("game"),
            source: SourceRef::new("src:game"),
            size: 123,
        });

        assert_eq!(content.kind(), ContentKind::Rom);
        assert_eq!(content.id().to_string(), "game");
    }

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
    fn returns_source_and_consumed_sources() {
        let content = Content::Disc(DiscContent {
            id: ContentId::new("game"),
            source: SourceRef::new("cue:/roms/game.cue"),
            title: "game".to_string(),
            disc_number: 1,
            consumed_sources: vec![SourceRef::new("cue:/roms/game.bin")],
        });

        assert_eq!(content.source().to_string(), "cue:/roms/game.cue");
        assert_eq!(content.consumed_sources().len(), 1);
        assert_eq!(
            content.consumed_sources()[0].to_string(),
            "cue:/roms/game.bin"
        );
    }

    #[test]
    fn converts_legacy_rom_content_to_decoded_rom_content() {
        let decoded = DecodedContent::from(Content::Rom(RomContent {
            id: ContentId::new("sonic"),
            source: SourceRef::new("file:/roms/sonic.bin"),
            size: 1024,
        }));

        match decoded {
            DecodedContent::Rom(rom) => {
                assert_eq!(rom.id.to_string(), "sonic");
                assert_eq!(rom.source.to_string(), "file:/roms/sonic.bin");
                assert_eq!(rom.size, 1024);
            }
            other => panic!("expected decoded rom content, got {other:?}"),
        }
    }

    #[test]
    fn rejects_pre_normalized_content_as_normalized_content() {
        let result = NormalizedContent::try_from(Content::Rom(RomContent {
            id: ContentId::new("sonic"),
            source: SourceRef::new("file:/roms/sonic.bin"),
            size: 1024,
        }));

        assert!(result.is_err());
    }

    #[test]
    fn formats_platform_as_expected() {
        assert_eq!(Platform::Snes.to_string(), "snes");
        assert_eq!(Platform::Ps1.to_string(), "ps1");
        assert_eq!(Platform::Nes.to_string(), "nes");
        assert_eq!(Platform::Megadrive.to_string(), "megadrive");
        assert_eq!(Platform::Unknown.to_string(), "unknown");
    }
}
