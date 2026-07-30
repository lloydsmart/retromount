use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::content::{ContentId, DecodedContent, DecodedDiscContent, DiscMedia, LogicalDisc};
use crate::core::reader_handle::ReaderHandle;
use crate::core::source::SourceObject;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;
use crate::readers::dir_reader::DirReader;

const ISO_SECTOR_SIZE: u32 = 2048;

/// Decodes a filesystem ISO using an explicit media hint supplied by the
/// application composition.
///
/// ISO contains a logical filesystem image, but does not reliably identify the
/// physical carrier from which it came. The decoder therefore must not infer
/// CD or DVD solely from the file extension or size.
#[derive(Debug, Clone, Copy)]
pub struct IsoDiscDecoder {
    media: DiscMedia,
}

impl IsoDiscDecoder {
    pub fn new(media: DiscMedia) -> Self {
        Self { media }
    }

    fn logical_disc(path: PathBuf, media: DiscMedia) -> io::Result<LogicalDisc> {
        let logical_bytes = fs::metadata(&path)?.len();

        if logical_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ISO image must not be empty",
            ));
        }
        if !logical_bytes.is_multiple_of(u64::from(ISO_SECTOR_SIZE)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("ISO size must contain whole {ISO_SECTOR_SIZE}-byte sectors"),
            ));
        }

        let reader_path = path.clone();
        let handle_id = format!("iso:{}", path.to_string_lossy());

        Ok(LogicalDisc {
            media,
            sector_size: ISO_SECTOR_SIZE,
            sector_count: logical_bytes / u64::from(ISO_SECTOR_SIZE),
            content: ReaderHandle::new(handle_id, move || {
                Ok(Box::new(DirReader::open(&reader_path)?))
            }),
        })
    }
}

impl InputDecoder for IsoDiscDecoder {
    fn supports(&self, identity: &InputIdentity) -> bool {
        matches!(identity, InputIdentity::IsoDisc)
    }

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<DecodedContent>, io::Error> {
        if !self.supports(identity) {
            return Ok(Vec::new());
        }

        let path = PathBuf::from(object.source.0.as_ref());
        let logical_disc = Self::logical_disc(path.clone(), self.media)?;
        let title = Path::new(&object.name)
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
    use crate::core::source::SourceRef;

    #[test]
    fn builds_a_live_logical_disc_from_valid_iso_geometry() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), vec![0; ISO_SECTOR_SIZE as usize * 3]).unwrap();

        let disc = IsoDiscDecoder::logical_disc(file.path().to_path_buf(), DiscMedia::Dvd).unwrap();

        assert_eq!(disc.media, DiscMedia::Dvd);
        assert_eq!(disc.sector_size, ISO_SECTOR_SIZE);
        assert_eq!(disc.sector_count, 3);
        assert_eq!(disc.byte_len(), Some(6144));
        assert!(disc.content.id().starts_with("iso:"));
    }

    #[test]
    fn rejects_empty_and_partial_sector_images() {
        let empty = tempfile::NamedTempFile::new().unwrap();
        let partial = tempfile::NamedTempFile::new().unwrap();
        fs::write(partial.path(), vec![0; ISO_SECTOR_SIZE as usize + 1]).unwrap();

        assert_eq!(
            IsoDiscDecoder::logical_disc(empty.path().to_path_buf(), DiscMedia::Dvd)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            IsoDiscDecoder::logical_disc(partial.path().to_path_buf(), DiscMedia::Dvd)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_with_the_explicit_media_hint() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), vec![0; ISO_SECTOR_SIZE as usize]).unwrap();
        let object = SourceObject {
            source: SourceRef::new(file.path().to_string_lossy().into_owned()),
            name: "Game.iso".to_string(),
        };

        let decoded = IsoDiscDecoder::new(DiscMedia::Dvd)
            .decode(&object, &InputIdentity::IsoDisc)
            .unwrap();
        let DecodedContent::Disc(disc) = &decoded[0] else {
            panic!("ISO should decode as a disc");
        };

        assert_eq!(
            disc.logical_disc.as_ref().map(|disc| disc.media),
            Some(DiscMedia::Dvd)
        );
    }
}
