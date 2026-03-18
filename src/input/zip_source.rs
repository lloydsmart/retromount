use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::source::{SourceObject, SourceRef};
use crate::input::source::InputSource;

#[derive(Debug, Clone)]
pub struct ZipInputSource {
    archive_path: PathBuf,
}

impl ZipInputSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            archive_path: path.into(),
        }
    }

    pub fn archive_path(&self) -> &Path {
        &self.archive_path
    }
}

impl InputSource for ZipInputSource {
    fn enumerate(&self) -> Result<Vec<SourceObject>, io::Error> {
        let file = File::open(&self.archive_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let mut objects = Vec::new();

        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

            if entry.is_dir() {
                continue;
            }

            let entry_name = entry.name().to_string();
            let display_name = Path::new(&entry_name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&entry_name)
                .to_string();

            let source = SourceRef::new(format!(
                "zip:{}#{}",
                self.archive_path.to_string_lossy(),
                entry_name
            ));

            objects.push(SourceObject {
                source,
                name: display_name,
            });
        }

        objects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(objects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn create_test_zip(path: &Path) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();

        zip.start_file("roms/sonic.bin", options).unwrap();
        zip.write_all(b"romdata").unwrap();

        zip.start_file("docs/readme.txt", options).unwrap();
        zip.write_all(b"hello").unwrap();

        zip.add_directory("empty_dir/", options).unwrap();

        zip.finish().unwrap();
    }

    #[test]
    fn enumerates_regular_files_in_zip_archive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let zip_path = temp_dir.path().join("test.zip");
        create_test_zip(&zip_path);

        let source = ZipInputSource::new(&zip_path);
        let objects = source.enumerate().unwrap();

        let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["readme.txt", "sonic.bin"]);

        assert!(objects[0].source.0.starts_with("zip:"));
        assert!(objects[1].source.0.starts_with("zip:"));
    }
}