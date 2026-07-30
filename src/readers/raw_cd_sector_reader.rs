use std::io;

use crate::core::reader::Reader;

const RAW_SECTOR_SIZE: u64 = 2352;
const LOGICAL_SECTOR_SIZE: u64 = 2048;
const MODE1_USER_OFFSET: usize = 16;
const MODE2_USER_OFFSET: usize = 24;
const CD_SYNC: [u8; 12] = [
    0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawCdDataMode {
    Mode1,
    Mode2Form1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawCdSectorKind {
    Mode1,
    Mode2Form1,
    Mode2Form2,
}

pub struct RawCdSectorReader {
    source: Box<dyn Reader>,
    source_offset: u64,
    sector_count: u64,
    mode: RawCdDataMode,
}

impl RawCdSectorReader {
    pub fn new(
        source: Box<dyn Reader>,
        source_offset: u64,
        sector_count: u64,
        mode: RawCdDataMode,
    ) -> io::Result<Self> {
        let encoded_len = sector_count
            .checked_mul(RAW_SECTOR_SIZE)
            .and_then(|length| source_offset.checked_add(length))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "raw CD range overflow"))?;

        if encoded_len > source.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "raw CD track extends past its source",
            ));
        }

        Ok(Self {
            source,
            source_offset,
            sector_count,
            mode,
        })
    }

    fn read_sector(&mut self, sector_index: u64, sector: &mut [u8; 2352]) -> io::Result<()> {
        let offset = self
            .source_offset
            .checked_add(sector_index * RAW_SECTOR_SIZE)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "raw CD offset overflow"))?;
        read_exact_at(self.source.as_mut(), offset, sector)?;
        let kind = validate_sector(sector, sector_index)?;
        match (self.mode, kind) {
            (RawCdDataMode::Mode1, RawCdSectorKind::Mode1)
            | (RawCdDataMode::Mode2Form1, RawCdSectorKind::Mode2Form1) => Ok(()),
            (RawCdDataMode::Mode1, _) => Err(invalid_sector(sector_index, "expected MODE1 sector")),
            (RawCdDataMode::Mode2Form1, RawCdSectorKind::Mode2Form2) => Err(invalid_sector(
                sector_index,
                "MODE2 Form 2 sector has 2324 user bytes and is not ISO-compatible",
            )),
            (RawCdDataMode::Mode2Form1, _) => {
                Err(invalid_sector(sector_index, "expected MODE2 sector"))
            }
        }
    }
}

pub fn validate_raw_cd_track(
    mut source: Box<dyn Reader>,
    source_offset: u64,
    sector_count: u64,
    mode: RawCdDataMode,
) -> io::Result<bool> {
    let end =
        source_offset
            .checked_add(sector_count.checked_mul(RAW_SECTOR_SIZE).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "raw CD range overflow")
            })?)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "raw CD range overflow"))?;
    if end > source.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "raw CD track extends past its source",
        ));
    }

    let mut sector = [0; RAW_SECTOR_SIZE as usize];
    let mut iso_compatible = true;

    for sector_index in 0..sector_count {
        read_exact_at(
            source.as_mut(),
            source_offset + sector_index * RAW_SECTOR_SIZE,
            &mut sector,
        )?;
        let kind = validate_sector(&sector, sector_index)?;

        match (mode, kind) {
            (RawCdDataMode::Mode1, RawCdSectorKind::Mode1)
            | (RawCdDataMode::Mode2Form1, RawCdSectorKind::Mode2Form1) => {}
            (RawCdDataMode::Mode2Form1, RawCdSectorKind::Mode2Form2) => {
                iso_compatible = false;
            }
            (RawCdDataMode::Mode1, _) => {
                return Err(invalid_sector(sector_index, "expected MODE1 sector"));
            }
            (RawCdDataMode::Mode2Form1, _) => {
                return Err(invalid_sector(sector_index, "expected MODE2 sector"));
            }
        }
    }

    Ok(iso_compatible)
}

impl Reader for RawCdSectorReader {
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || offset >= self.len() {
            return Ok(0);
        }

        let requested = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        let read_len = requested.min(self.len() - offset) as usize;
        let mut written = 0;
        let mut logical_offset = offset;
        let mut sector = [0; RAW_SECTOR_SIZE as usize];

        while written < read_len {
            let sector_index = logical_offset / LOGICAL_SECTOR_SIZE;
            let within_sector = (logical_offset % LOGICAL_SECTOR_SIZE) as usize;
            self.read_sector(sector_index, &mut sector)?;

            let user_offset = match self.mode {
                RawCdDataMode::Mode1 => MODE1_USER_OFFSET,
                RawCdDataMode::Mode2Form1 => MODE2_USER_OFFSET,
            };
            let available = LOGICAL_SECTOR_SIZE as usize - within_sector;
            let count = available.min(read_len - written);
            let source_start = user_offset + within_sector;
            buffer[written..written + count]
                .copy_from_slice(&sector[source_start..source_start + count]);

            written += count;
            logical_offset += count as u64;
        }

        Ok(written)
    }

    fn len(&self) -> u64 {
        self.sector_count * LOGICAL_SECTOR_SIZE
    }
}

fn read_exact_at(reader: &mut dyn Reader, offset: u64, buffer: &mut [u8]) -> io::Result<()> {
    let mut read = 0;

    while read < buffer.len() {
        let count = reader.read_at(offset + read as u64, &mut buffer[read..])?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "raw CD sector is truncated",
            ));
        }
        read += count;
    }

    Ok(())
}

fn validate_sector(sector: &[u8; 2352], sector_index: u64) -> io::Result<RawCdSectorKind> {
    if sector[..CD_SYNC.len()] != CD_SYNC {
        return Err(invalid_sector(sector_index, "invalid sync pattern"));
    }

    match sector[15] {
        1 => Ok(RawCdSectorKind::Mode1),
        2 if sector[16..20] != sector[20..24] => Err(invalid_sector(
            sector_index,
            "MODE2 subheader copies do not match",
        )),
        2 if sector[18] & 0x20 != 0 => Ok(RawCdSectorKind::Mode2Form2),
        2 => Ok(RawCdSectorKind::Mode2Form1),
        _ => Err(invalid_sector(sector_index, "unknown sector mode")),
    }
}

fn invalid_sector(sector_index: u64, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("raw CD sector {sector_index}: {message}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::inline_reader::InlineReader;

    fn raw_sector(mode: RawCdDataMode, fill: u8) -> Vec<u8> {
        let mut sector = vec![0; RAW_SECTOR_SIZE as usize];
        sector[..CD_SYNC.len()].copy_from_slice(&CD_SYNC);

        match mode {
            RawCdDataMode::Mode1 => {
                sector[15] = 1;
                sector[MODE1_USER_OFFSET..MODE1_USER_OFFSET + LOGICAL_SECTOR_SIZE as usize]
                    .fill(fill);
            }
            RawCdDataMode::Mode2Form1 => {
                sector[15] = 2;
                sector[16..20].copy_from_slice(&[1, 2, 0, 4]);
                sector[20..24].copy_from_slice(&[1, 2, 0, 4]);
                sector[MODE2_USER_OFFSET..MODE2_USER_OFFSET + LOGICAL_SECTOR_SIZE as usize]
                    .fill(fill);
            }
        }

        sector
    }

    #[test]
    fn maps_unaligned_reads_across_mode1_sector_boundaries() {
        let mut encoded = raw_sector(RawCdDataMode::Mode1, 0x11);
        encoded.extend(raw_sector(RawCdDataMode::Mode1, 0x22));
        let mut reader = RawCdSectorReader::new(
            Box::new(InlineReader::new(encoded)),
            0,
            2,
            RawCdDataMode::Mode1,
        )
        .unwrap();
        let mut output = [0; 8];

        assert_eq!(reader.read_at(2044, &mut output).unwrap(), 8);
        assert_eq!(&output, &[0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]);
        assert_eq!(reader.len(), 4096);
    }

    #[test]
    fn maps_mode2_form1_user_data() {
        let encoded = raw_sector(RawCdDataMode::Mode2Form1, 0x5a);
        let mut reader = RawCdSectorReader::new(
            Box::new(InlineReader::new(encoded)),
            0,
            1,
            RawCdDataMode::Mode2Form1,
        )
        .unwrap();
        let mut output = [0; 4];

        assert_eq!(reader.read_at(7, &mut output).unwrap(), 4);
        assert_eq!(output, [0x5a; 4]);
    }

    #[test]
    fn rejects_mode2_form2_sectors() {
        let mut encoded = raw_sector(RawCdDataMode::Mode2Form1, 0x5a);
        encoded[18] |= 0x20;
        encoded[22] |= 0x20;
        let mut reader = RawCdSectorReader::new(
            Box::new(InlineReader::new(encoded)),
            0,
            1,
            RawCdDataMode::Mode2Form1,
        )
        .unwrap();

        let error = reader.read_at(0, &mut [0; 1]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("Form 2"));
    }

    #[test]
    fn rejects_invalid_sync_and_truncated_ranges() {
        let invalid = vec![0; RAW_SECTOR_SIZE as usize];
        let mut reader = RawCdSectorReader::new(
            Box::new(InlineReader::new(invalid)),
            0,
            1,
            RawCdDataMode::Mode1,
        )
        .unwrap();
        assert!(reader.read_at(0, &mut [0; 1]).is_err());

        let error = match RawCdSectorReader::new(
            Box::new(InlineReader::new(vec![0; 100])),
            0,
            1,
            RawCdDataMode::Mode1,
        ) {
            Ok(_) => panic!("truncated raw track should fail"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }
}
