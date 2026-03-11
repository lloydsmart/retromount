use std::io::ErrorKind;
use std::path::Path;

use crate::core::input_registry::InputRegistry;
use crate::core::payload_filter::filter_payload_files;
use crate::core::virtual_file::VirtualFile;
use crate::error::RetromountError;

pub struct Loader {
    registry: InputRegistry,
}

impl Loader {
    pub fn new(registry: InputRegistry) -> Self {
        Self { registry }
    }

    pub fn discover_path(&self, path: &Path) -> Result<Vec<VirtualFile>, RetromountError> {
        let files = self.registry.discover(path).map_err(|err| {
            if err.kind() == ErrorKind::Unsupported {
                RetromountError::UnsupportedFormat
            } else {
                RetromountError::LoadError(err.to_string())
            }
        })?;

        Ok(filter_payload_files(files))
    }
}

impl Default for Loader {
    fn default() -> Self {
        Self::new(InputRegistry::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{Builder, NamedTempFile, TempDir};
    use zip::write::SimpleFileOptions;

    #[test]
    fn discovers_single_file_using_default_registry() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        tmp.write_all(b"retromount")
            .expect("failed to write test data");

        let loader = Loader::default();
        let files = loader.discover_path(tmp.path()).expect("discovery failed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size, 10);
    }

    #[test]
    fn discovers_directory_contents_using_default_registry() {
        let dir = TempDir::new().expect("failed to create temp dir");

        std::fs::write(dir.path().join("a.bin"), b"aaa").expect("failed to write first file");
        std::fs::write(dir.path().join("b.bin"), b"bbb").expect("failed to write second file");

        let loader = Loader::default();
        let files = loader.discover_path(dir.path()).expect("discovery failed");

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn discovers_zip_contents_using_default_registry() {
        let mut tmp = Builder::new()
            .suffix(".zip")
            .tempfile()
            .expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = SimpleFileOptions::default();

            zip.start_file("game.sfc", options)
                .expect("failed to start first zip entry");
            zip.write_all(b"game-data")
                .expect("failed to write first zip entry");

            zip.start_file("readme.txt", options)
                .expect("failed to start second zip entry");
            zip.write_all(b"readme-data")
                .expect("failed to write second zip entry");

            zip.finish().expect("failed to finish zip");
        }

        let loader = Loader::default();
        let mut files = loader.discover_path(tmp.path()).expect("discovery failed");

        files.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "game.sfc");
    }

    #[test]
    fn returns_unsupported_format_when_registry_has_no_matching_handler() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        tmp.write_all(b"retromount")
            .expect("failed to write test data");

        let loader = Loader::new(InputRegistry::new());
        let result = loader.discover_path(tmp.path());

        assert!(matches!(result, Err(RetromountError::UnsupportedFormat)));
    }

    #[test]
    fn filters_ancillary_files_from_zip_contents() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".zip")
            .tempfile()
            .expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = zip::write::SimpleFileOptions::default();

            zip.start_file("game.sfc", options)
                .expect("failed to start first zip entry");
            zip.write_all(b"game-data")
                .expect("failed to write first zip entry");

            zip.start_file("readme.txt", options)
                .expect("failed to start second zip entry");
            zip.write_all(b"readme-data")
                .expect("failed to write second zip entry");

            zip.start_file("release.nfo", options)
                .expect("failed to start third zip entry");
            zip.write_all(b"nfo-data")
                .expect("failed to write third zip entry");

            zip.finish().expect("failed to finish zip");
        }

        let loader = Loader::default();
        let files = loader.discover_path(tmp.path()).expect("discovery failed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "game.sfc");
    }

    #[test]
    fn discovers_cue_referenced_files_using_default_registry() {
        let dir = tempfile::TempDir::new().expect("failed to create temp dir");

        let cue_path = dir.path().join("game.cue");
        let bin_path = dir.path().join("track01.bin");

        std::fs::write(
            &cue_path,
            r#"
    FILE "track01.bin" BINARY
    TRACK 01 MODE1/2352
        INDEX 01 00:00:00
    "#,
        )
        .expect("failed to write cue file");

        std::fs::write(&bin_path, b"fake-bin-data").expect("failed to write bin file");

        let loader = Loader::default();
        let files = loader.discover_path(&cue_path).expect("discovery failed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "track01.bin");
        assert_eq!(files[0].origin, bin_path);
    }
}
