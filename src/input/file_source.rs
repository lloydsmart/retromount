use std::io;
use std::path::{Path, PathBuf};

use crate::core::source::{SourceObject, SourceOrigin, SourceRef};
use crate::core::source_resolver::filesystem_content;
use crate::input::source::{InputSource, InputSourceKind};

#[derive(Debug, Clone)]
pub struct FileInputSource {
    path: PathBuf,
}

impl FileInputSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl InputSource for FileInputSource {
    fn kind(&self) -> InputSourceKind {
        InputSourceKind::File
    }

    fn enumerate(&self) -> Result<Vec<SourceObject>, io::Error> {
        let name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| self.path.to_string_lossy().into_owned());

        Ok(vec![SourceObject {
            source: SourceRef::new(self.path.to_string_lossy().into_owned()),
            name,
            origin: SourceOrigin::Filesystem(self.path.clone()),
            content: filesystem_content(&self.path)?,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_file_kind() {
        let source = FileInputSource::new("/tmp/game.cue");
        assert_eq!(source.kind(), InputSourceKind::File);
    }

    #[test]
    fn enumerates_single_file_object() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("game.cue");
        std::fs::write(&path, b"FILE \"game.bin\" BINARY").unwrap();
        let source = FileInputSource::new(&path);
        let objects = source.enumerate().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].name, "game.cue");
        assert_eq!(objects[0].source.to_string(), path.to_string_lossy());
    }
}
