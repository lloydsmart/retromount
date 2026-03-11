use std::fs::File;
use std::io::{Read, Result, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::core::reader::Reader;

pub struct ZipReader {
    archive_path: PathBuf,
    entry_name: String,
    size: u64,
    is_stored: bool,
    data_start: u64,
}

impl ZipReader {
    pub fn open(archive_path: &Path, entry_name: &str) -> Result<Self> {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let entry = archive.by_name(entry_name)?;

        let is_stored = entry.compression() == zip::CompressionMethod::Stored;
        let data_start = entry.data_start();

        Ok(Self {
            archive_path: archive_path.to_path_buf(),
            entry_name: entry_name.to_string(),
            size: entry.size(),
            is_stored,
            data_start,
        })
    }
}

impl Reader for ZipReader {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        if offset >= self.size || buf.is_empty() {
            return Ok(0);
        }

        if self.is_stored {
            let mut file = File::open(&self.archive_path)?;
            file.seek(SeekFrom::Start(self.data_start + offset))?;
            return file.read(buf);
        }

        let file = File::open(&self.archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut entry = archive.by_name(&self.entry_name)?;

        // Compressed ZIP entries are stream-oriented, so we cannot seek
        // directly to an offset in the decompressed data. Instead, discard
        // bytes up to the requested offset, then read the requested window.
        let mut skipped = std::io::copy(&mut entry.by_ref().take(offset), &mut std::io::sink())?;

        while skipped < offset {
            let just_skipped = std::io::copy(
                &mut entry.by_ref().take(offset - skipped),
                &mut std::io::sink(),
            )?;

            if just_skipped == 0 {
                return Ok(0);
            }

            skipped += just_skipped;
        }

        entry.read(buf)
    }

    fn len(&self) -> u64 {
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

        assert_eq!(reader.len(), 14);

        let mut buf = vec![0u8; 14];
        let bytes = reader
            .read_at(0, &mut buf)
            .expect("failed to read zip entry");

        assert_eq!(bytes, 14);
        assert_eq!(&buf, b"retromount-zip");
    }

    #[test]
    fn test_zip_reader_reads_stored_entry_from_offset() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zip.start_file("stored.rom", options)
                .expect("failed to start zip entry");
            zip.write_all(b"abcdefghijklmnopqrstuvwxyz")
                .expect("failed to write zip entry");
            zip.finish().expect("failed to finish zip");
        }

        let mut reader =
            ZipReader::open(tmp.path(), "stored.rom").expect("failed to open zip reader");

        let mut buf = vec![0u8; 5];
        let bytes = reader
            .read_at(5, &mut buf)
            .expect("failed to read stored zip entry from offset");

        assert_eq!(bytes, 5);
        assert_eq!(&buf, b"fghij");
    }

    #[test]
    fn test_zip_reader_reads_compressed_entry_from_offset() {
        let mut tmp = NamedTempFile::new().expect("failed to create temp zip");

        {
            let mut zip = zip::ZipWriter::new(&mut tmp);
            let options = SimpleFileOptions::default();

            zip.start_file("compressed.rom", options)
                .expect("failed to start zip entry");
            zip.write_all(b"abcdefghijklmnopqrstuvwxyz")
                .expect("failed to write zip entry");
            zip.finish().expect("failed to finish zip");
        }

        let mut reader =
            ZipReader::open(tmp.path(), "compressed.rom").expect("failed to open zip reader");

        let mut buf = vec![0u8; 5];
        let bytes = reader
            .read_at(5, &mut buf)
            .expect("failed to read compressed zip entry from offset");

        assert_eq!(bytes, 5);
        assert_eq!(&buf, b"fghij");
    }
}
