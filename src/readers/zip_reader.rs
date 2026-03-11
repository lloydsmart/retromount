use std::fs::File;
use std::io::{Read, Result};
use std::path::{Path, PathBuf};

use crate::core::reader::Reader;
use crate::core::reader_factory::ReaderFactory;

pub struct ZipReader {
    archive_path: PathBuf,
    entry_name: String,
    size: u64,
}

impl ZipReader {
    pub fn open(archive_path: &Path, entry_name: &str) -> Result<Self> {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let entry = archive.by_name(entry_name)?;

        Ok(Self {
            archive_path: archive_path.to_path_buf(),
            entry_name: entry_name.to_string(),
            size: entry.size(),
        })
    }
}

impl Reader for ZipReader {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let file = File::open(&self.archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut entry = archive.by_name(&self.entry_name)?;

        let mut data = Vec::with_capacity(self.size as usize);
        entry.read_to_end(&mut data)?;

        let start = offset as usize;
        if start >= data.len() {
            return Ok(0);
        }

        let end = (start + buf.len()).min(data.len());
        let slice = &data[start..end];
        buf[..slice.len()].copy_from_slice(slice);

        Ok(slice.len())
    }

    fn size(&self) -> u64 {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader::Reader;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_zip_reader_reads_entry_contents() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = SimpleFileOptions::default();

            zip.start_file("test.rom", options)
                .expect("failed to start zip entry");
            zip.write_all(b"retromount-zip")
                .expect("failed to write zip entry");
            zip.finish().expect("failed to finish zip");
        }

        let mut reader =
            ZipReader::open(tmp.path(), "test.rom").expect("failed to open zip reader");

        assert_eq!(reader.size(), 14);

        let mut buf = vec![0u8; 14];
        let bytes = reader
            .read_at(0, &mut buf)
            .expect("failed to read zip entry");

        assert_eq!(bytes, 14);
        assert_eq!(&buf, b"retromount-zip");
    }
}
