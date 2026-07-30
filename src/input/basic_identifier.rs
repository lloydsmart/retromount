use std::io;
use std::path::Path;

use crate::core::source::SourceObject;
use crate::input::identify::{InputIdentifier, InputIdentity};

#[derive(Debug, Default)]
pub struct BasicInputIdentifier;

impl BasicInputIdentifier {
    pub fn new() -> Self {
        Self
    }
}

impl InputIdentifier for BasicInputIdentifier {
    fn identify(&self, object: &SourceObject) -> Result<InputIdentity, io::Error> {
        let path = Path::new(&object.name);

        if path.is_dir() {
            return Ok(InputIdentity::Directory);
        }

        let identity = match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("cue") => InputIdentity::DiscImage,
            Some(ext) if ext.eq_ignore_ascii_case("chd") => InputIdentity::ChdDisc,
            Some(ext) if ext.eq_ignore_ascii_case("iso") => InputIdentity::IsoDisc,
            Some(ext)
                if ext.eq_ignore_ascii_case("txt")
                    || ext.eq_ignore_ascii_case("md")
                    || ext.eq_ignore_ascii_case("nfo") =>
            {
                InputIdentity::Text
            }
            Some(_) => InputIdentity::File,
            None => InputIdentity::Unknown,
        };

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input_content::{InputAccess, InputContent};
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::{SourceObject, SourceOrigin, SourceRef};
    use crate::readers::inline_reader::InlineReader;

    fn object(name: &str) -> SourceObject {
        SourceObject {
            source: SourceRef::new(format!("/tmp/{name}")),
            name: name.to_string(),
            origin: SourceOrigin::Filesystem(format!("/tmp/{name}").into()),
            content: InputContent::new(
                0,
                InputAccess::RandomAccess,
                ReaderHandle::new(format!("test:{name}"), || {
                    Ok(Box::new(InlineReader::new(Vec::new())))
                }),
            ),
        }
    }

    #[test]
    fn identifies_text_file() {
        let identifier = BasicInputIdentifier::new();
        let object = object("readme.txt");

        let identity = identifier.identify(&object).unwrap();
        assert_eq!(identity, InputIdentity::Text);
    }

    #[test]
    fn identifies_disc_image() {
        let identifier = BasicInputIdentifier::new();
        let object = object("game.cue");

        let identity = identifier.identify(&object).unwrap();
        assert_eq!(identity, InputIdentity::DiscImage);
    }

    #[test]
    fn identifies_chd_as_distinct_disc_container() {
        let identifier = BasicInputIdentifier::new();
        let object = object("game.CHD");

        assert_eq!(
            identifier.identify(&object).unwrap(),
            InputIdentity::ChdDisc
        );
    }

    #[test]
    fn identifies_iso_as_a_distinct_disc_format() {
        let identifier = BasicInputIdentifier::new();
        let object = object("game.ISO");

        assert_eq!(
            identifier.identify(&object).unwrap(),
            InputIdentity::IsoDisc
        );
    }
}
