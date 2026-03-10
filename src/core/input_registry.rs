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
                return handler.discover(path);
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
        Self::new()
    }
}
