use std::io;

use crate::core::reader::Reader;

pub struct InterleavedSectorReader {
    source: Box<dyn Reader>,
    source_offset: u64,
    sector_count: u64,
    input_sector_size: u64,
    output_offset: u64,
    output_sector_size: u64,
}

impl InterleavedSectorReader {
    pub fn new(
        source: Box<dyn Reader>,
        source_offset: u64,
        sector_count: u64,
        input_sector_size: u64,
        output_offset: u64,
        output_sector_size: u64,
    ) -> io::Result<Self> {
        let output_end = output_offset
            .checked_add(output_sector_size)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "sector range overflow"))?;
        if input_sector_size == 0 || output_sector_size == 0 || output_end > input_sector_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid interleaved sector geometry",
            ));
        }
        let source_len = sector_count
            .checked_mul(input_sector_size)
            .and_then(|length| source_offset.checked_add(length))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "sector range overflow"))?;
        if source_len > source.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "interleaved sector range extends past its source",
            ));
        }
        Ok(Self {
            source,
            source_offset,
            sector_count,
            input_sector_size,
            output_offset,
            output_sector_size,
        })
    }
}

impl Reader for InterleavedSectorReader {
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        let length = self.len();
        if buffer.is_empty() || offset >= length {
            return Ok(0);
        }

        let requested = buffer
            .len()
            .min(usize::try_from(length - offset).unwrap_or(usize::MAX));
        let mut written = 0;
        while written < requested {
            let logical_offset = offset + written as u64;
            let sector = logical_offset / self.output_sector_size;
            let within = logical_offset % self.output_sector_size;
            let count = (requested - written)
                .min(usize::try_from(self.output_sector_size - within).unwrap_or(usize::MAX));
            let source_offset =
                self.source_offset + sector * self.input_sector_size + self.output_offset + within;
            let read = self
                .source
                .read_at(source_offset, &mut buffer[written..written + count])?;
            written += read;
            if read < count {
                break;
            }
        }
        Ok(written)
    }

    fn len(&self) -> u64 {
        self.sector_count * self.output_sector_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::inline_reader::InlineReader;

    #[test]
    fn projects_unaligned_reads_across_interleaved_sectors() {
        let source = InlineReader::new(b"abcdXYefghZZ".to_vec());
        let mut reader = InterleavedSectorReader::new(Box::new(source), 0, 2, 6, 0, 4).unwrap();
        let mut output = [0; 6];

        assert_eq!(reader.read_at(2, &mut output).unwrap(), 6);
        assert_eq!(&output, b"cdefgh");
        assert_eq!(reader.len(), 8);
    }
}
