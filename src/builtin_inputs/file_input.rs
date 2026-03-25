use std::path::Path;

use crate::core::input_handler::InputHandler;
use crate::core::reader_registry::ReaderRegistry;
use crate::core::virtual_file::VirtualFile;

/// Input handler for regular files.
///
/// This is the simplest input case:
/// one filesystem file becomes one VirtualFile.
pub struct FileInputHandler {
    reader_registry: ReaderRegistry,
}

impl FileInputHandler {
    pub fn new(reader_registry: ReaderRegistry) -> Self {
        Self { reader_registry }
    }
}

impl InputHandler for FileInputHandler {
    fn supports(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn discover(
        &self,
        _registry: &crate::core::input_registry::InputRegistry,
        path: &Path,
    ) -> std::io::Result<Vec<VirtualFile>> {
        let reader = self.reader_registry.open(path)?;
        let size = reader.len();

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        Ok(vec![VirtualFile::new(
            name,
            size,
            path.to_path_buf(),
            reader,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn file_input_handler_discovers_single_virtual_file() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        let data = b"retromount-file-input";
        tmp.write_all(data).expect("failed to write test data");

        let handler = FileInputHandler::new(ReaderRegistry::default());
        let registry = crate::core::input_registry::InputRegistry::default();
        let files = handler
            .discover(&registry, tmp.path())
            .expect("discovery failed");

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
