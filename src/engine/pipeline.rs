use std::collections::HashSet;
use std::io;

use crate::core::content::{ContentMeta, DecodedContent, NormalizedContent};
use crate::core::normalizer::{normalize_decoded_content, NormalizationOptions};
use crate::core::source::SourceObject;
use crate::core::vfs::VfsDirectory;
use crate::input::decode::InputDecoder;
use crate::input::identify::{InputIdentifier, InputIdentity};
use crate::input::source::InputSource;
use crate::output::present::OutputPresenter;
use crate::policy::PolicySet;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PipelineTrace {
    pub objects: Vec<TracedObject>,
    pub normalized: Vec<NormalizedContent>,
    pub output_vfs: VfsDirectory,
}

#[derive(Debug, Clone, Serialize)]
pub struct TracedObject {
    pub object: SourceObject,
    pub identity: InputIdentity,
    pub supported: bool,
    pub decoded: Vec<DecodedContent>,
}

pub fn run_pipeline(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
) -> Result<VfsDirectory, io::Error> {
    Ok(run_pipeline_with_options(
        source,
        identifier,
        decoder,
        presenter,
        policy,
        &NormalizationOptions::default(),
    )?
    .output_vfs)
}

pub fn run_pipeline_with_trace(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
) -> Result<PipelineTrace, io::Error> {
    run_pipeline_with_options(
        source,
        identifier,
        decoder,
        presenter,
        policy,
        &NormalizationOptions::default(),
    )
}

pub fn run_pipeline_with_options(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
    normalization_options: &NormalizationOptions,
) -> Result<PipelineTrace, io::Error> {
    let objects = source.enumerate()?;
    let mut traced_objects = Vec::new();
    let mut all_decoded_content = Vec::new();

    for object in objects {
        let identity = identifier.identify(&object)?;
        let supported = decoder.supports(&identity);

        let decoded = if supported {
            decoder.decode(&object, &identity)?
        } else {
            Vec::new()
        };

        all_decoded_content.extend(decoded.iter().cloned());

        traced_objects.push(TracedObject {
            object,
            identity,
            supported,
            decoded,
        });
    }

    let normalized = normalize_decoded_content(all_decoded_content, normalization_options);
    let normalized_presentable_content = suppress_consumed_content(&normalized);
    let output_vfs = presenter.present(&normalized_presentable_content, policy);

    Ok(PipelineTrace {
        objects: traced_objects,
        normalized,
        output_vfs,
    })
}

fn suppress_consumed_content(all_content: &[NormalizedContent]) -> Vec<NormalizedContent> {
    let consumed_sources: HashSet<_> = all_content
        .iter()
        .flat_map(|content| content.consumed_sources().iter().cloned())
        .collect();

    all_content
        .iter()
        .filter(|content| !consumed_sources.contains(content.source()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::core::content::{DecodedContentKind, NormalizedContentKind};
    use crate::input::basic_decoder::BasicInputDecoder;
    use crate::input::basic_identifier::BasicInputIdentifier;
    use crate::input::directory_source::DirectoryInputSource;
    use crate::output::flat_presenter::FlatPresenter;
    use crate::output::grouped_presenter::GroupedPresenter;

    #[test]
    fn runs_end_to_end_directory_pipeline() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("mario.sfc"), b"rom").unwrap();
        fs::write(temp_dir.path().join("readme.txt"), b"hello").unwrap();
        fs::write(temp_dir.path().join("blob.dat"), b"xyz").unwrap();

        let source = DirectoryInputSource::new(temp_dir.path());
        let identifier = BasicInputIdentifier::new();
        let decoder = BasicInputDecoder::new();
        let presenter = GroupedPresenter::new();

        let policy = PolicySet::default();
        let root = run_pipeline(&source, &identifier, &decoder, &presenter, &policy).unwrap();

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["snes", "blob.dat.bin", "readme.txt"]);
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
        let presenter = GroupedPresenter::new();
        let policy = PolicySet::default();
        let root = run_pipeline(&source, &identifier, &decoder, &presenter, &policy).unwrap();

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["unknown", "blob.dat.bin", "readme.txt"]);
    }

    #[test]
    fn traces_end_to_end_directory_pipeline() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("mario.sfc"), b"rom").unwrap();
        fs::write(temp_dir.path().join("readme.txt"), b"hello").unwrap();
        fs::write(temp_dir.path().join("blob.dat"), b"xyz").unwrap();

        let source = DirectoryInputSource::new(temp_dir.path());
        let identifier = BasicInputIdentifier::new();
        let decoder = BasicInputDecoder::new();
        let presenter = GroupedPresenter::new();

        let policy = PolicySet::default();
        let trace =
            run_pipeline_with_trace(&source, &identifier, &decoder, &presenter, &policy).unwrap();

        assert_eq!(trace.objects.len(), 3);
        assert_eq!(trace.normalized.len(), 3);

        let names: Vec<&str> = trace
            .output_vfs
            .children
            .iter()
            .map(|node| node.name())
            .collect();
        assert_eq!(names, vec!["snes", "blob.dat.bin", "readme.txt"]);

        assert_eq!(trace.objects[0].object.name, "blob.dat");
        assert_eq!(trace.objects[0].identity, InputIdentity::File);
        assert!(trace.objects[0].supported);
        assert_eq!(trace.objects[0].decoded.len(), 1);
        assert_eq!(
            trace.objects[0].decoded[0].kind(),
            DecodedContentKind::Bytes
        );

        assert_eq!(trace.objects[1].object.name, "mario.sfc");
        assert_eq!(trace.objects[1].identity, InputIdentity::File);
        assert!(trace.objects[1].supported);
        assert_eq!(trace.objects[1].decoded.len(), 1);
        assert_eq!(trace.objects[1].decoded[0].kind(), DecodedContentKind::Rom);
        assert_eq!(trace.normalized[1].kind(), NormalizedContentKind::Game);

        assert_eq!(trace.objects[2].object.name, "readme.txt");
        assert_eq!(trace.objects[2].identity, InputIdentity::Text);
        assert!(trace.objects[2].supported);
        assert_eq!(trace.objects[2].decoded.len(), 1);
        assert_eq!(trace.objects[2].decoded[0].kind(), DecodedContentKind::Text);
        assert_eq!(trace.normalized[2].kind(), NormalizedContentKind::Text);

        assert_eq!(trace.normalized[0].kind(), NormalizedContentKind::Bytes);
        assert_eq!(trace.normalized[1].kind(), NormalizedContentKind::Game);
        assert_eq!(trace.normalized[2].kind(), NormalizedContentKind::Text);
    }

    #[test]
    fn renders_different_vfs_layouts_for_grouped_and_flat_presenters() {
        let temp = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(temp.path().join("roms/snes")).unwrap();
        std::fs::write(
            temp.path().join("roms/snes/Super Mario World.sfc"),
            b"smw-data",
        )
        .unwrap();

        let source = crate::input::directory_source::DirectoryInputSource::new(temp.path());
        let identifier = crate::input::basic_identifier::BasicInputIdentifier::new();
        let decoder = crate::input::basic_decoder::BasicInputDecoder::new();

        let grouped = GroupedPresenter::new();
        let flat = FlatPresenter::new();

        let policy = PolicySet::default();
        let grouped_root = run_pipeline(&source, &identifier, &decoder, &grouped, &policy).unwrap();
        let flat_root = run_pipeline(&source, &identifier, &decoder, &flat, &policy).unwrap();

        assert!(grouped_root.find_directory("snes").is_some());
        assert!(grouped_root.find_directory("ps1").is_none());
        assert!(grouped_root
            .find_directory("snes/Super Mario World")
            .is_some());
        assert!(grouped_root
            .find_file("snes/Super Mario World/Super Mario World.sfc")
            .is_some());

        assert!(flat_root.find_directory("snes").is_none());
        assert!(flat_root.find_directory("ps1").is_none());
        assert!(flat_root.find_file("Super Mario World.sfc").is_some());
    }
}
