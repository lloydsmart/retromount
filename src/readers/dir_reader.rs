use std::fs::File;
use std::io::{Read, Result, Seek, SeekFrom};
use std::path::Path;

use crate::core::reader::Reader;
use crate::core::reader_factory::ReaderFactory;

pub struct DirReader {
    file: File,
    size: u64,
}

impl DirReader {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let size = file.metadata()?.len();

        Ok(Self { file, size })
    }
}

impl Reader for DirReader {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read(buf)
    }

    fn size(&self) -> u64 {
        self.size
    }
}

pub struct DirReaderFactory;

impl ReaderFactory for DirReaderFactory {
    fn supports(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn create(&self, path: &Path) -> Result<Box<dyn Reader>> {
        Ok(Box::new(DirReader::open(path)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader::Reader;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_dir_reader_reads_file_contents() {
        // Create a temporary file
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");

        let data = b"retromount-test-data";
        tmp.write_all(data).expect("failed to write test data");

        // Open reader
        let mut reader = DirReader::open(tmp.path()).expect("failed to open reader");

        // Verify size
        assert_eq!(reader.size(), data.len() as u64);

        // Read data
        let mut buf = vec![0u8; data.len()];
        let bytes = reader.read_at(0, &mut buf).expect("read failed");

        assert_eq!(bytes, data.len());
        assert_eq!(buf, data);
    }
}
