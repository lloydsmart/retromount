use std::io;
use std::path::Path;

use crate::core::content::{ContentId, DecodedContent, DecodedDiscContent, DiscMedia, LogicalDisc};
use crate::core::input_content::InputContent;
use crate::core::source::SourceObject;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;

const ISO_SECTOR_SIZE: u32 = 2048;

/// Decodes a random-access ISO using an explicit media hint supplied by the
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

    fn logical_disc(content: &InputContent, media: DiscMedia) -> io::Result<LogicalDisc> {
        content.open_random_access()?;
        let logical_bytes = content.size;

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

        Ok(LogicalDisc {
            media,
            sector_size: ISO_SECTOR_SIZE,
            sector_count: logical_bytes / u64::from(ISO_SECTOR_SIZE),
            content: content.handle.clone(),
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

        let logical_disc = Self::logical_disc(&object.content, self.media)?;
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
    use std::fs;

    use crate::input::file_source::FileInputSource;
    use crate::input::source::InputSource;

    fn object_for(path: &Path, name: &str) -> SourceObject {
        let mut object = FileInputSource::new(path).enumerate().unwrap().remove(0);
        object.name = name.to_string();
        object
    }

    #[test]
    fn builds_a_live_logical_disc_from_valid_iso_geometry() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), vec![0; ISO_SECTOR_SIZE as usize * 3]).unwrap();

        let object = object_for(file.path(), "Game.iso");
        let disc = IsoDiscDecoder::logical_disc(&object.content, DiscMedia::Dvd).unwrap();

        assert_eq!(disc.media, DiscMedia::Dvd);
        assert_eq!(disc.sector_size, ISO_SECTOR_SIZE);
        assert_eq!(disc.sector_count, 3);
        assert_eq!(disc.byte_len(), Some(6144));
        assert!(disc.content.id().starts_with("file:"));
    }

    #[test]
    fn rejects_empty_and_partial_sector_images() {
        let empty = tempfile::NamedTempFile::new().unwrap();
        let partial = tempfile::NamedTempFile::new().unwrap();
        fs::write(partial.path(), vec![0; ISO_SECTOR_SIZE as usize + 1]).unwrap();

        assert_eq!(
            IsoDiscDecoder::logical_disc(
                &object_for(empty.path(), "Empty.iso").content,
                DiscMedia::Dvd
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            IsoDiscDecoder::logical_disc(
                &object_for(partial.path(), "Partial.iso").content,
                DiscMedia::Dvd,
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_with_the_explicit_media_hint() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), vec![0; ISO_SECTOR_SIZE as usize]).unwrap();
        let object = object_for(file.path(), "Game.iso");

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
