use std::io;

use crate::core::reader::Reader;

pub struct RangeReader {
    source: Box<dyn Reader>,
    offset: u64,
    length: u64,
}

impl RangeReader {
    pub fn new(source: Box<dyn Reader>, offset: u64, length: u64) -> io::Result<Self> {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "reader range overflow"))?;
        if end > source.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "reader range extends past its source",
            ));
        }

        Ok(Self {
            source,
            offset,
            length,
        })
    }
}

impl Reader for RangeReader {
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || offset >= self.length {
            return Ok(0);
        }

        let available = usize::try_from(self.length - offset).unwrap_or(usize::MAX);
        let requested = buffer.len().min(available);
        self.source
            .read_at(self.offset + offset, &mut buffer[..requested])
    }

    fn len(&self) -> u64 {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::inline_reader::InlineReader;

    #[test]
    fn exposes_only_the_selected_source_range() {
        let mut reader =
            RangeReader::new(Box::new(InlineReader::new(b"0123456789".to_vec())), 3, 4).unwrap();
        let mut output = [0; 8];

        assert_eq!(reader.read_at(1, &mut output).unwrap(), 3);
        assert_eq!(&output[..3], b"456");
        assert_eq!(reader.len(), 4);
        assert_eq!(reader.read_at(4, &mut output).unwrap(), 0);
    }
}
