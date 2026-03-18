use std::io;

use crate::core::vfs::VfsDirectory;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentifier;
use crate::input::source::InputSource;
use crate::output::present::OutputPresenter;

pub fn run_pipeline(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
) -> Result<VfsDirectory, io::Error> {
    let objects = source.enumerate()?;
    let mut content = Vec::new();

    for object in objects {
        let identity = identifier.identify(&object)?;

        if decoder.supports(&identity) {
            content.extend(decoder.decode(&object, &identity)?);
        }
    }

    Ok(presenter.present(&content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::input::basic_decoder::BasicInputDecoder;
    use crate::input::basic_identifier::BasicInputIdentifier;
    use crate::input::directory_source::DirectoryInputSource;
    use crate::output::basic_encoder::BasicEncoder;
    use crate::output::generic_presenter::GenericPresenter;

    #[test]
    fn runs_end_to_end_directory_pipeline() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("mario.sfc"), b"rom").unwrap();
        fs::write(temp_dir.path().join("readme.txt"), b"hello").unwrap();
        fs::write(temp_dir.path().join("blob.dat"), b"xyz").unwrap();

        let source = DirectoryInputSource::new(temp_dir.path());
        let identifier = BasicInputIdentifier::new();
        let decoder = BasicInputDecoder::new();
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let root = run_pipeline(&source, &identifier, &decoder, &presenter).unwrap();

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["blob.dat.bin", "mario.sfc", "readme.txt"]);
    }
}
