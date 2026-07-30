use std::fs::File;
use std::io::Write;

use retromount::core::content::Platform;
use retromount::core::normalizer::NormalizationOptions;
use retromount::core::vfs_resolver::open_file;
use retromount::engine::components::pipeline_components;
use retromount::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use retromount::input::zip_source::ZipInputSource;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

const SECTOR_SIZE: usize = 2048;
const LOGICAL_SIZE: usize = SECTOR_SIZE * 3;

#[test]
fn presents_a_stored_zip_iso_through_the_live_opl_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let zip_path = temp_dir.path().join("library.zip");
    write_iso_zip(&zip_path, CompressionMethod::Stored);

    let trace = run_opl(&zip_path).unwrap();
    let file = trace
        .output_vfs
        .find_file("DVD/Test Game.iso")
        .expect("stored ZIP ISO should be present");
    assert_eq!(file.size, LOGICAL_SIZE as u64);

    let mut reader = open_file(&trace.output_vfs, "DVD/Test Game.iso").unwrap();
    let mut bytes = [0; 31];
    let offset = SECTOR_SIZE - 11;
    assert_eq!(
        reader.read_at(offset as u64, &mut bytes).unwrap(),
        bytes.len()
    );
    assert_eq!(
        bytes,
        std::array::from_fn(|index| fixture_byte(offset + index))
    );
}

#[test]
fn rejects_a_compressed_zip_iso_that_cannot_supply_efficient_random_access() {
    let temp_dir = tempfile::tempdir().unwrap();
    let zip_path = temp_dir.path().join("library.zip");
    write_iso_zip(&zip_path, CompressionMethod::Deflated);

    let error = run_opl(&zip_path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    assert!(error
        .to_string()
        .contains("does not provide efficient random access"));
}

fn run_opl(
    zip_path: &std::path::Path,
) -> std::io::Result<retromount::engine::pipeline::PipelineTrace> {
    let components = pipeline_components("opl").unwrap();
    let source = ZipInputSource::new(zip_path);

    run_pipeline_with_presentation_options(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &PipelineOptions {
            normalization: NormalizationOptions {
                platform_hint: Some(Platform::Ps2),
            },
            plugin_registry: None,
        },
    )
}

fn write_iso_zip(path: &std::path::Path, compression: CompressionMethod) {
    let file = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(compression);
    zip.start_file("ps2/Test Game.iso", options).unwrap();
    zip.write_all(&(0..LOGICAL_SIZE).map(fixture_byte).collect::<Vec<_>>())
        .unwrap();
    zip.finish().unwrap();
}

fn fixture_byte(offset: usize) -> u8 {
    ((offset * 31 + 5) % 251) as u8
}
