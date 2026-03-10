use std::io::Result;
use std::path::Path;

use crate::core::virtual_file::VirtualFile;

/// An InputHandler converts a path into one or more VirtualFiles.
///
/// Different handlers support different input formats:
/// - regular files
/// - ZIP archives
/// - directories
/// - CHD disc images
/// - CUE/BIN sets
pub trait InputHandler: Send + Sync {
    /// Returns true if this handler supports the given path.
    fn supports(&self, path: &Path) -> bool;

    /// Discover VirtualFiles from the path.
    fn discover(&self, path: &Path) -> Result<Vec<VirtualFile>>;
}
