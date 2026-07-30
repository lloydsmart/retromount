use std::io::{self, Read, Seek, SeekFrom};

use crate::core::reader::Reader;

/// Adapts Retromount's offset-based reader contract to `Read + Seek` consumers.
pub struct ReaderCursor {
    reader: Box<dyn Reader>,
    position: u64,
}

impl ReaderCursor {
    pub fn new(reader: Box<dyn Reader>) -> Self {
        Self {
            reader,
            position: 0,
        }
    }
}

impl Read for ReaderCursor {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.reader.read_at(self.position, buffer)?;
        self.position = self
            .position
            .checked_add(bytes_read as u64)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "reader position overflow")
            })?;
        Ok(bytes_read)
    }
}

impl Seek for ReaderCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let next = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            SeekFrom::End(offset) => i128::from(self.reader.len()) + i128::from(offset),
        };

        self.position = u64::try_from(next).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot seek before the start of input",
            )
        })?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::inline_reader::InlineReader;

    #[test]
    fn adapts_offset_reads_to_read_and_seek() {
        let mut cursor = ReaderCursor::new(Box::new(InlineReader::new(b"abcdef".to_vec())));
        let mut bytes = [0; 3];

        cursor.seek(SeekFrom::Start(2)).unwrap();
        cursor.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"cde");

        cursor.seek(SeekFrom::End(-2)).unwrap();
        cursor.read_exact(&mut bytes[..2]).unwrap();
        assert_eq!(&bytes[..2], b"ef");
    }
}
