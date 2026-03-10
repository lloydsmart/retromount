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
    use std::fs::File;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

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

        let file_a = dir.path().join("a.bin");
        let file_b = dir.path().join("b.bin");

        File::create(&file_a)
            .expect("failed to create first file")
            .write_all(b"aaa")
            .expect("failed to write first file");

        File::create(&file_b)
            .expect("failed to create second file")
            .write_all(b"bbb")
            .expect("failed to write second file");

        let registry = InputRegistry::default();
        let files = registry.discover(dir.path()).expect("discovery failed");

        assert_eq!(files.len(), 2);
    }
}
