use serde::Serialize;
use std::fmt;
use std::sync::Arc;

use crate::core::source::SourceRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ContentKind {
    Bytes,
    Rom,
    Disc,
    Text,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Content {
    Bytes(BytesContent),
    Rom(RomContent),
    Disc(DiscContent),
    Text(TextContent),
}

impl Content {
    pub fn kind(&self) -> ContentKind {
        match self {
            Self::Bytes(_) => ContentKind::Bytes,
            Self::Rom(_) => ContentKind::Rom,
            Self::Disc(_) => ContentKind::Disc,
            Self::Text(_) => ContentKind::Text,
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
pub struct BytesContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RomContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub file_name: String,
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
    fn returns_content_kind() {
        let content = Content::Rom(RomContent {
            id: ContentId::new("game"),
            source: SourceRef::new("src:game"),
            file_name: "game.sfc".to_string(),
            size: 123,
        });

        assert_eq!(content.kind(), ContentKind::Rom);
        assert_eq!(content.id().to_string(), "game");
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
}
