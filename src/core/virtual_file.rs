use std::path::PathBuf;

use crate::core::reader::Reader;

/// Represents a virtual file exposed by RetroMount.
///
/// A VirtualFile maps a visible filename to a Reader implementation
/// that provides the file's contents. The `origin` records the input
/// source path this VirtualFile was derived from.
pub struct VirtualFile {
    pub name: String,
    pub size: u64,
    pub origin: PathBuf,
    pub reader: Box<dyn Reader>,
}

impl VirtualFile {
    pub fn new(name: String, size: u64, origin: PathBuf, reader: Box<dyn Reader>) -> Self {
        Self {
            name,
            size,
            origin,
            reader,
        }
    }
}
