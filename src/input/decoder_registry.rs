use std::io;

use crate::core::content::DecodedContent;
use crate::core::source::SourceObject;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;

#[derive(Default)]
pub struct DecoderRegistry {
    decoders: Vec<Box<dyn InputDecoder>>,
}

impl DecoderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, decoder: impl InputDecoder + 'static) {
        self.decoders.push(Box::new(decoder));
    }
}

impl InputDecoder for DecoderRegistry {
    fn supports(&self, identity: &InputIdentity) -> bool {
        self.decoders
            .iter()
            .any(|decoder| decoder.supports(identity))
    }

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<DecodedContent>, io::Error> {
        let decoder = self
            .decoders
            .iter()
            .find(|decoder| decoder.supports(identity))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("no decoder supports {identity:?}"),
                )
            })?;

        decoder.decode(object, identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input_content::{InputAccess, InputContent};
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::{SourceOrigin, SourceRef};
    use crate::input::basic_decoder::BasicInputDecoder;
    use crate::readers::dir_reader::DirReader;

    fn object_for(path: &std::path::Path, name: &str) -> SourceObject {
        let reader_path = path.to_path_buf();
        SourceObject {
            source: SourceRef::new(path.to_string_lossy().into_owned()),
            name: name.to_string(),
            origin: SourceOrigin::Filesystem(path.to_path_buf()),
            content: InputContent::new(
                std::fs::metadata(path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0),
                InputAccess::RandomAccess,
                ReaderHandle::new(format!("test:{}", path.display()), move || {
                    Ok(Box::new(DirReader::open(&reader_path)?))
                }),
            ),
        }
    }

    #[test]
    fn delegates_to_a_decoder_that_supports_the_identity() {
        let mut registry = DecoderRegistry::new();
        registry.register(BasicInputDecoder::new());
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = object_for(file.path(), "readme.txt");

        let decoded = registry.decode(&object, &InputIdentity::Text).unwrap();

        assert_eq!(decoded.len(), 1);
    }

    #[test]
    fn rejects_an_identity_without_a_registered_decoder() {
        let registry = DecoderRegistry::new();
        let file = tempfile::NamedTempFile::new().unwrap();
        let object = object_for(file.path(), "game.chd");

        let error = registry
            .decode(&object, &InputIdentity::ChdDisc)
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }
}
