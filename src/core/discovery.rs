use std::io::Result;
use std::path::Path;

use crate::core::reader_registry::ReaderRegistry;
use crate::core::virtual_file::VirtualFile;

/// Discover one or more VirtualFiles from a path.
///
/// For now:
/// - regular files produce a single VirtualFile
/// - directories are not yet expanded
pub fn discover_virtual_files(registry: &ReaderRegistry, path: &Path) -> Result<Vec<VirtualFile>> {
    let reader = registry.open(path)?;
    let size = reader.size();

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    Ok(vec![VirtualFile::new(name, size, reader)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader::Reader;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn discovers_single_virtual_file_from_regular_file() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        let data = b"retromount";
        tmp.write_all(data).expect("failed to write test data");

        let registry = ReaderRegistry::default();
        let files = discover_virtual_files(&registry, tmp.path()).expect("discovery failed");

        assert_eq!(files.len(), 1);

        // `VirtualFile` owns a `Box<dyn Reader>`. The `read_at()` method on the
        // `Reader` trait requires `&mut self`, so we must bind the VirtualFile
        // as mutable in order to call `read_at()` on its reader.
        let mut vf = files.into_iter().next().expect("missing virtual file");

        assert_eq!(vf.size, data.len() as u64);

        let expected_name = tmp
            .path()
            .file_name()
            .expect("temp file missing name")
            .to_string_lossy()
            .into_owned();

        assert_eq!(vf.name, expected_name);

        let mut buf = vec![0u8; data.len()];
        let bytes = vf.reader.read_at(0, &mut buf).expect("read failed");

        assert_eq!(bytes, data.len());
        assert_eq!(buf, data);
    }
}
