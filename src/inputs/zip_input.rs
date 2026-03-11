use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::core::input_handler::InputHandler;
use crate::core::input_registry::InputRegistry;
use crate::core::virtual_file::VirtualFile;
use crate::readers::zip_reader::ZipReader;

/// Input handler for ZIP archives.
///
/// A ZIP archive is expanded into one VirtualFile per file entry.
/// Each VirtualFile is backed by a ZipReader for that specific entry.
pub struct ZipInputHandler;

fn has_zip_magic(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return false;
    }

    matches!(
        magic,
        [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08]
    )
}

impl InputHandler for ZipInputHandler {
    fn supports(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        path.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            || has_zip_magic(path)
    }

    fn discover(
        &self,
        _registry: &InputRegistry,
        path: &Path,
    ) -> std::io::Result<Vec<VirtualFile>> {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut files = Vec::new();

        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;

            if !entry.is_file() {
                continue;
            }

            let entry_name = entry.name().to_string();
            let size = entry.size();
            let reader = Box::new(ZipReader::open(path, &entry_name)?);

            files.push(VirtualFile::new(
                entry_name,
                size,
                path.to_path_buf(),
                reader,
            ));
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{Builder, NamedTempFile};
    use zip::write::SimpleFileOptions;

    #[test]
    fn zip_input_handler_discovers_files_in_zip_archive() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = SimpleFileOptions::default();

            zip.start_file("game.sfc", options)
                .expect("failed to start first zip entry");
            zip.write_all(b"game-data")
                .expect("failed to write first zip entry");

            zip.start_file("manual.txt", options)
                .expect("failed to start second zip entry");
            zip.write_all(b"manual-data")
                .expect("failed to write second zip entry");

            zip.finish().expect("failed to finish zip");
        }

        let registry = InputRegistry::default();
        let handler = ZipInputHandler;

        let mut files = handler
            .discover(&registry, tmp.path())
            .expect("discovery failed");

        files.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "game.sfc");
        assert_eq!(files[0].size, 9);
        assert_eq!(files[1].name, "manual.txt");
        assert_eq!(files[1].size, 11);
    }

    #[test]
    fn zip_input_handler_supports_zip_magic_without_zip_extension() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = SimpleFileOptions::default();

            zip.start_file("game.sfc", options)
                .expect("failed to start zip entry");
            zip.write_all(b"game-data")
                .expect("failed to write zip entry");

            zip.finish().expect("failed to finish zip");
        }

        let handler = ZipInputHandler;
        assert!(handler.supports(tmp.path()));
    }

    #[test]
    fn zip_input_handler_rejects_non_zip_file() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        tmp.write_all(b"not-a-zip")
            .expect("failed to write test data");

        let handler = ZipInputHandler;
        assert!(!handler.supports(tmp.path()));
    }
}
