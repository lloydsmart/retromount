use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceRef(pub Arc<str>);

impl SourceRef {
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for SourceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObject {
    pub source: SourceRef,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_source_ref() {
        let source = SourceRef::new("zip:/roms/snes.zip#game.sfc");
        assert_eq!(source.to_string(), "zip:/roms/snes.zip#game.sfc");
    }
}
