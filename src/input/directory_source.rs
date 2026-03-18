use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::source::{SourceObject, SourceRef};
use crate::input::source::InputSource;

#[derive(Debug, Clone)]
pub struct DirectoryInputSource {
    root: PathBuf,
}

impl DirectoryInputSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl InputSource for DirectoryInputSource {
    fn enumerate(&self) -> Result<Vec<SourceObject>, io::Error> {
        let mut objects = Vec::new();

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let source = SourceRef::new(path.to_string_lossy().into_owned());

                objects.push(SourceObject { source, name });
            }
        }

        objects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(objects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn enumerates_regular_files_in_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("b.txt"), b"hello").unwrap();
        fs::write(temp_dir.path().join("a.bin"), b"abc").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let source = DirectoryInputSource::new(temp_dir.path());
        let objects = source.enumerate().unwrap();

        let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["a.bin", "b.txt"]);
    }
}
