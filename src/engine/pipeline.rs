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

    #[test]
    fn runs_end_to_end_zip_pipeline() {
        use std::fs::File;
        use std::io::Write;

        use crate::input::zip_source::ZipInputSource;

        let temp_dir = tempfile::tempdir().unwrap();
        let zip_path = temp_dir.path().join("library.zip");

        let file = File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();

        zip.start_file("roms/sonic.bin", options).unwrap();
        zip.write_all(b"rom").unwrap();

        zip.start_file("docs/readme.txt", options).unwrap();
        zip.write_all(b"hello").unwrap();

        zip.start_file("misc/blob.dat", options).unwrap();
        zip.write_all(b"xyz").unwrap();

        zip.finish().unwrap();

        let source = ZipInputSource::new(&zip_path);
        let identifier = BasicInputIdentifier::new();
        let decoder = BasicInputDecoder::new();
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let root = run_pipeline(&source, &identifier, &decoder, &presenter).unwrap();

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["blob.dat.bin", "readme.txt", "sonic.bin"]);
    }
}
