use std::io;

use chd::metadata::MetadataTag;
use chd::{Chd, Error as ChdError};

use crate::core::content::{ContentId, DecodedContent, DecodedDiscContent, DiscMedia, LogicalDisc};
use crate::core::input_content::InputContent;
use crate::core::reader_cursor::ReaderCursor;
use crate::core::source::SourceObject;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;
use crate::readers::chd_reader::ChdReader;

const DVD_METADATA_TAG: u32 = u32::from_be_bytes(*b"DVD ");
const DVD_SECTOR_SIZE: u32 = 2048;

#[derive(Debug, Default)]
pub struct ChdDiscDecoder;

#[derive(Debug, Clone, Copy)]
struct ChdDiscInfo {
    has_parent: bool,
    has_dvd_metadata: bool,
    unit_bytes: u32,
    logical_bytes: u64,
}

impl ChdDiscDecoder {
    pub fn new() -> Self {
        Self
    }

    fn inspect(content: &InputContent) -> io::Result<ChdDiscInfo> {
        let cursor = ReaderCursor::new(content.open_random_access()?);
        let mut chd = Chd::open(cursor, None).map_err(map_chd_open_error)?;
        let header = chd.header();
        let has_parent = header.has_parent();
        let unit_bytes = header.unit_bytes();
        let logical_bytes = header.logical_bytes();
        let has_dvd_metadata = chd
            .metadata_refs()
            .any(|metadata| metadata.metatag() == DVD_METADATA_TAG);

        Ok(ChdDiscInfo {
            has_parent,
            has_dvd_metadata,
            unit_bytes,
            logical_bytes,
        })
    }

    fn logical_disc(content: &InputContent, info: ChdDiscInfo) -> io::Result<LogicalDisc> {
        if info.has_parent {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "parent/delta CHDs are not supported",
            ));
        }
        if !info.has_dvd_metadata {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "CHD does not describe DVD media",
            ));
        }
        if info.unit_bytes != DVD_SECTOR_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "DVD CHD unit size must be {DVD_SECTOR_SIZE} bytes, found {}",
                    info.unit_bytes
                ),
            ));
        }
        if info.logical_bytes == 0 || !info.logical_bytes.is_multiple_of(DVD_SECTOR_SIZE.into()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DVD CHD logical size must contain whole 2048-byte sectors",
            ));
        }

        let handle = content.handle.clone();
        Ok(LogicalDisc {
            media: DiscMedia::Dvd,
            sector_size: DVD_SECTOR_SIZE,
            sector_count: info.logical_bytes / u64::from(DVD_SECTOR_SIZE),
            content: crate::core::reader_handle::ReaderHandle::new(
                format!("chd:{}", content.handle.id()),
                move || Ok(Box::new(ChdReader::open(&handle)?)),
            ),
        })
    }
}

fn map_chd_open_error(error: ChdError) -> io::Error {
    let kind = match error {
        ChdError::InvalidFile
        | ChdError::InvalidData
        | ChdError::InvalidMetadata
        | ChdError::InvalidMetadataSize
        | ChdError::ReadError => io::ErrorKind::InvalidData,
        ChdError::RequiresParent
        | ChdError::NotSupported
        | ChdError::UnsupportedFormat
        | ChdError::UnsupportedVersion => io::ErrorKind::Unsupported,
        _ => io::ErrorKind::Other,
    };

    io::Error::new(kind, format!("failed to open CHD: {error}"))
}

impl InputDecoder for ChdDiscDecoder {
    fn supports(&self, identity: &InputIdentity) -> bool {
        matches!(identity, InputIdentity::ChdDisc)
    }

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<DecodedContent>, io::Error> {
        if !self.supports(identity) {
            return Ok(Vec::new());
        }

        let logical_disc = Self::logical_disc(&object.content, Self::inspect(&object.content)?)?;
        let title = std::path::Path::new(&object.name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&object.name)
            .to_string();

        Ok(vec![DecodedContent::Disc(DecodedDiscContent {
            id: ContentId::new(object.name.clone()),
            source: object.source.clone(),
            title,
            disc_number: 1,
            consumed_sources: Vec::new(),
            logical_disc: Some(logical_disc),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input_content::{InputAccess, InputContent};
    use crate::core::reader_handle::ReaderHandle;
    use crate::readers::inline_reader::InlineReader;

    fn test_content() -> InputContent {
        InputContent::new(
            1,
            InputAccess::RandomAccess,
            ReaderHandle::new("test:chd", || Ok(Box::new(InlineReader::new(vec![0])))),
        )
    }

    fn valid_info() -> ChdDiscInfo {
        ChdDiscInfo {
            has_parent: false,
            has_dvd_metadata: true,
            unit_bytes: DVD_SECTOR_SIZE,
            logical_bytes: u64::from(DVD_SECTOR_SIZE) * 3,
        }
    }

    #[test]
    fn builds_a_live_logical_dvd_from_valid_geometry() {
        let disc = ChdDiscDecoder::logical_disc(&test_content(), valid_info()).unwrap();

        assert_eq!(disc.media, DiscMedia::Dvd);
        assert_eq!(disc.sector_size, DVD_SECTOR_SIZE);
        assert_eq!(disc.sector_count, 3);
        assert_eq!(disc.byte_len(), Some(6144));
        assert_eq!(disc.content.id(), "chd:test:chd");
    }

    #[test]
    fn rejects_parent_and_non_dvd_chds() {
        let parent = ChdDiscInfo {
            has_parent: true,
            ..valid_info()
        };
        let non_dvd = ChdDiscInfo {
            has_dvd_metadata: false,
            ..valid_info()
        };

        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), parent)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), non_dvd)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_non_iso_dvd_geometry() {
        let wrong_unit = ChdDiscInfo {
            unit_bytes: 4096,
            ..valid_info()
        };
        let partial_sector = ChdDiscInfo {
            logical_bytes: 2049,
            ..valid_info()
        };

        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), wrong_unit)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), partial_sector)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn maps_invalid_and_unsupported_chd_errors_to_actionable_io_kinds() {
        assert_eq!(
            map_chd_open_error(ChdError::InvalidData).kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            map_chd_open_error(ChdError::UnsupportedVersion).kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            map_chd_open_error(ChdError::RequiresParent).kind(),
            io::ErrorKind::Unsupported
        );
    }
}
