use crate::core::reader::Reader;

/// Represents a virtual file exposed by RetroMount.
///
/// A VirtualFile maps a filename to a Reader implementation
/// that provides the file's contents.
pub struct VirtualFile {
    pub name: String,
    pub size: u64,
    pub reader: Box<dyn Reader>,
}

impl VirtualFile {
    pub fn new(name: String, size: u64, reader: Box<dyn Reader>) -> Self {
        Self { name, size, reader }
    }
}
