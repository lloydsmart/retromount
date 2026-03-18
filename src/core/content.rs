use std::fmt;
use std::sync::Arc;

use crate::core::source::SourceRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentKind {
    Bytes,
    Rom,
    Disc,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytesContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscContent {
    pub id: ContentId,
    pub source: SourceRef,
    pub title: String,
    pub disc_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
}
