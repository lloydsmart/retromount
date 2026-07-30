use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::source::{SourceObject, SourceOrigin, SourceRef};
use crate::core::source_resolver::filesystem_content;
use crate::input::source::{InputSource, InputSourceKind};

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

    fn collect_files(
        root: &Path,
        current: &Path,
        objects: &mut Vec<SourceObject>,
    ) -> Result<(), io::Error> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::collect_files(root, &path, objects)?;
                continue;
            }

            if path.is_file() {
                let relative = path.strip_prefix(root).unwrap_or(&path);

                let name = relative
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");

                let source = SourceRef::new(path.to_string_lossy().into_owned());

                objects.push(SourceObject {
                    source,
                    name,
                    origin: SourceOrigin::Filesystem(path.clone()),
                    content: filesystem_content(&path)?,
                });
            }
        }

        Ok(())
    }
}

impl InputSource for DirectoryInputSource {
    fn kind(&self) -> InputSourceKind {
        InputSourceKind::Directory
    }

    fn enumerate(&self) -> Result<Vec<SourceObject>, io::Error> {
        let mut objects = Vec::new();
        Self::collect_files(&self.root, &self.root, &mut objects)?;
        objects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(objects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reports_directory_kind() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = DirectoryInputSource::new(temp_dir.path());

        assert_eq!(source.kind(), InputSourceKind::Directory);
    }

    #[test]
    fn enumerates_regular_files_in_directory_recursively() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("b.txt"), b"hello").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir").join("a.bin"), b"abc").unwrap();

        let source = DirectoryInputSource::new(temp_dir.path());
        let objects = source.enumerate().unwrap();

        let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["b.txt", "subdir/a.bin"]);
    }

    #[test]
    fn ignores_directories_and_sorts_nested_results() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp_dir.path().join("roms/snes")).unwrap();
        fs::create_dir_all(temp_dir.path().join("roms/megadrive")).unwrap();

        fs::write(temp_dir.path().join("roms/snes/zelda.sfc"), b"1").unwrap();
        fs::write(temp_dir.path().join("roms/megadrive/sonic.bin"), b"2").unwrap();

        let source = DirectoryInputSource::new(temp_dir.path());
        let objects = source.enumerate().unwrap();

        let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["roms/megadrive/sonic.bin", "roms/snes/zelda.sfc"]
        );
    }
}
