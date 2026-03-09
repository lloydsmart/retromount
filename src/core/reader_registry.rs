use std::io::{Error, ErrorKind, Result};
use std::path::Path;

use super::reader::Reader;
use super::reader_factory::ReaderFactory;

pub struct ReaderRegistry {
    factories: Vec<Box<dyn ReaderFactory>>,
}

impl ReaderRegistry {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    pub fn register(&mut self, factory: Box<dyn ReaderFactory>) {
        self.factories.push(factory);
    }

    pub fn open(&self, path: &Path) -> Result<Box<dyn Reader>> {
        for factory in &self.factories {
            if factory.supports(path) {
                return factory.create(path);
            }
        }

        Err(Error::new(
            ErrorKind::Unsupported,
            format!("No reader available for {}", path.display()),
        ))
    }
}

impl Default for ReaderRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Register built-in readers
        registry.register(Box::new(crate::readers::dir_reader::DirReaderFactory));

        registry
    }
}
