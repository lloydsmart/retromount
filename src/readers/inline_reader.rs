use std::io::Result;

use crate::core::reader::Reader;

pub struct InlineReader {
    data: Vec<u8>,
}

impl InlineReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Reader for InlineReader {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let offset = offset as usize;

        if offset >= self.data.len() || buf.is_empty() {
            return Ok(0);
        }

        let available = &self.data[offset..];
        let count = available.len().min(buf.len());
        buf[..count].copy_from_slice(&available[..count]);

        Ok(count)
    }

    fn len(&self) -> u64 {
        self.data.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader::Reader;

    #[test]
    fn reads_inline_bytes() {
        let mut reader = InlineReader::new(b"abcdef".to_vec());

        let mut buf = vec![0; 3];
        let bytes = reader.read_at(2, &mut buf).unwrap();

        assert_eq!(bytes, 3);
        assert_eq!(&buf, b"cde");
    }

    #[test]
    fn returns_zero_when_offset_is_past_end() {
        let mut reader = InlineReader::new(b"abc".to_vec());

        let mut buf = vec![0; 3];
        let bytes = reader.read_at(10, &mut buf).unwrap();

        assert_eq!(bytes, 0);
    }
}
