use std::io::Result;
use std::path::Path;

use super::reader::Reader;

/// Factory trait used to create Reader implementations.
///
/// Each ReaderFactory knows how to detect whether it can
/// handle a particular path and create the appropriate reader.
pub trait ReaderFactory: Send + Sync {
    /// Returns true if this factory supports the given path.
    fn supports(&self, path: &Path) -> bool;

    /// Create a reader for the path.
    fn create(&self, path: &Path) -> Result<Box<dyn Reader>>;
}
