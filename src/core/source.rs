use serde::Serialize;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SourceRef(pub Arc<str>);

impl SourceRef {
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    pub fn file_name(&self) -> String {
        let normalized = self.0.replace('\\', "/");

        let leaf = match normalized.rsplit_once('#') {
            Some((_, member)) => member,
            None => normalized.rsplit('/').next().unwrap_or(&normalized),
        };

        leaf.to_string()
    }
}

impl fmt::Display for SourceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

    #[test]
    fn derives_file_name_from_filesystem_source() {
        let source = SourceRef::new("file:/roms/Super Mario World.sfc");
        assert_eq!(source.file_name(), "Super Mario World.sfc");
    }

    #[test]
    fn derives_file_name_from_zip_member_source() {
        let source = SourceRef::new("zip:/roms/megadrive.zip#Sonic the Hedgehog.bin");
        assert_eq!(source.file_name(), "Sonic the Hedgehog.bin");
    }
}
