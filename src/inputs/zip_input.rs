use std::fs::File;
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

impl InputHandler for ZipInputHandler {
    fn supports(&self, path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
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

            files.push(VirtualFile::new(entry_name, size, reader));
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
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
}
