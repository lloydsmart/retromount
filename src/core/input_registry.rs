use std::io::{Error, ErrorKind, Result};
use std::path::Path;

use crate::core::input_handler::InputHandler;
use crate::core::virtual_file::VirtualFile;

/// Registry of available input handlers.
///
/// An InputRegistry is responsible for selecting the first handler that
/// supports a given path and delegating discovery to it.
pub struct InputRegistry {
    handlers: Vec<Box<dyn InputHandler>>,
}

impl InputRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Box<dyn InputHandler>) {
        self.handlers.push(handler);
    }

    pub fn discover(&self, path: &Path) -> Result<Vec<VirtualFile>> {
        for handler in &self.handlers {
            if handler.supports(path) {
                return handler.discover(self, path);
            }
        }

        Err(Error::new(
            ErrorKind::Unsupported,
            format!("No input handler available for {}", path.display()),
        ))
    }
}

impl Default for InputRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        crate::inputs::register_builtin_inputs(&mut registry);
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};
    use zip::write::SimpleFileOptions;

    #[test]
    fn input_registry_discovers_single_file() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        tmp.write_all(b"retromount")
            .expect("failed to write test data");

        let registry = InputRegistry::default();
        let files = registry.discover(tmp.path()).expect("discovery failed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size, 10);
    }

    #[test]
    fn input_registry_discovers_directory_contents() {
        let dir = TempDir::new().expect("failed to create temp dir");

        std::fs::write(dir.path().join("a.bin"), b"aaa").expect("failed to write first file");
        std::fs::write(dir.path().join("b.bin"), b"bbb").expect("failed to write second file");

        let registry = InputRegistry::default();
        let files = registry.discover(dir.path()).expect("discovery failed");

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn input_registry_discovers_zip_contents() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = SimpleFileOptions::default();

            zip.start_file("game.sfc", options).unwrap();
            zip.write_all(b"game-data").unwrap();

            zip.start_file("readme.txt", options).unwrap();
            zip.write_all(b"readme-data").unwrap();

            zip.finish().unwrap();
        }

        let registry = InputRegistry::default();
        let mut files = registry.discover(tmp.path()).expect("discovery failed");

        files.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "game.sfc");
        assert_eq!(files[1].name, "readme.txt");
    }
}
