use serde::Serialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::input_content::InputContent;

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
    pub origin: SourceOrigin,
    pub content: InputContent,
}

impl SourceObject {
    pub fn resolve_relative(&self, referenced_path: &Path) -> SourceRef {
        self.origin.resolve_relative(referenced_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SourceOrigin {
    Filesystem(PathBuf),
    ZipEntry {
        archive_path: PathBuf,
        entry_name: String,
    },
}

impl SourceOrigin {
    fn resolve_relative(&self, referenced_path: &Path) -> SourceRef {
        match self {
            Self::Filesystem(path) => {
                let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
                SourceRef::new(
                    base_dir
                        .join(referenced_path)
                        .to_string_lossy()
                        .into_owned(),
                )
            }
            Self::ZipEntry {
                archive_path,
                entry_name,
            } => {
                let base_dir = Path::new(entry_name)
                    .parent()
                    .unwrap_or_else(|| Path::new(""));
                let resolved = normalize_virtual_path(base_dir.join(referenced_path));
                SourceRef::new(format!("zip:{}#{resolved}", archive_path.to_string_lossy()))
            }
        }
    }
}

fn normalize_virtual_path(path: PathBuf) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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

    #[test]
    fn resolves_relative_paths_from_structured_origins() {
        let filesystem = SourceOrigin::Filesystem(PathBuf::from("/roms/ps1/game.cue"));
        assert_eq!(
            filesystem
                .resolve_relative(Path::new("game.bin"))
                .to_string(),
            "/roms/ps1/game.bin"
        );

        let zip = SourceOrigin::ZipEntry {
            archive_path: PathBuf::from("/roms/ps1.zip"),
            entry_name: "games/game.cue".to_string(),
        };
        assert_eq!(
            zip.resolve_relative(Path::new("game.bin")).to_string(),
            "zip:/roms/ps1.zip#games/game.bin"
        );
    }
}
