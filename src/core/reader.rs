use std::io::Result;

/// Trait implemented by all RetroMount input readers.
///
/// A Reader provides random-access reads over some underlying
/// storage format (filesystem, ZIP archive, CHD, ISO, etc).
///
/// Readers are always read-only.
pub trait Reader: Send + Sync {
    /// Read data starting at `offset` into `buf`.
    ///
    /// Returns number of bytes read.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize>;

    /// Return the total length of the underlying data stream.
    fn len(&self) -> u64;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
