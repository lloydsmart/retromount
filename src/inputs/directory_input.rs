use std::fs;
use std::path::Path;

use crate::core::input_handler::InputHandler;
use crate::core::input_registry::InputRegistry;
use crate::core::virtual_file::VirtualFile;

/// Input handler for directories.
///
/// A directory is expanded by delegating discovery of each entry
/// back through the InputRegistry.
pub struct DirectoryInputHandler;

impl InputHandler for DirectoryInputHandler {
    fn supports(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn discover(&self, registry: &InputRegistry, path: &Path) -> std::io::Result<Vec<VirtualFile>> {
        let mut files = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            let mut discovered = registry.discover(&entry_path)?;
            files.append(&mut discovered);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader::Reader;
    use crate::core::reader_registry::ReaderRegistry;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn directory_input_handler_discovers_files_in_directory() {
        let dir = TempDir::new().expect("failed to create temp dir");

        let file_a = dir.path().join("a.bin");
        let file_b = dir.path().join("b.bin");

        File::create(&file_a).unwrap().write_all(b"aaa").unwrap();

        File::create(&file_b).unwrap().write_all(b"bbb").unwrap();

        let registry = crate::core::input_registry::InputRegistry::default();
        let handler = DirectoryInputHandler;

        let files = handler
            .discover(&registry, dir.path())
            .expect("discovery failed");

        assert_eq!(files.len(), 2);
    }
}
