use std::fs;

use retromount::core::content::Platform;
use retromount::core::normalizer::NormalizationOptions;
use retromount::core::vfs_resolver::open_file;
use retromount::engine::components::pipeline_components;
use retromount::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use retromount::input::file_source::FileInputSource;

const HEADER_SIZE: usize = 124;
const HUNK_SIZE: usize = 4096;
const SECTOR_SIZE: usize = 2048;
const HUNK_COUNT: usize = 2;
const LOGICAL_SIZE: usize = HUNK_SIZE * HUNK_COUNT;
const MAP_OFFSET: usize = HEADER_SIZE;
const METADATA_OFFSET: usize = MAP_OFFSET + HUNK_COUNT * 4;
const DATA_OFFSET: usize = HUNK_SIZE;

#[test]
fn presents_a_dvd_chd_as_a_live_opl_iso() {
    let temp_dir = tempfile::tempdir().unwrap();
    let chd_path = temp_dir.path().join("Test Game.chd");
    fs::write(&chd_path, dvd_chd_fixture()).unwrap();

    let components = pipeline_components("opl").unwrap();
    let source = FileInputSource::new(&chd_path);
    let options = PipelineOptions {
        normalization: NormalizationOptions {
            platform_hint: Some(Platform::Ps2),
        },
        plugin_registry: None,
    };

    let trace = run_pipeline_with_presentation_options(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &options,
    )
    .unwrap();

    assert!(trace.objects[0].supported);
    let file = trace
        .output_vfs
        .find_file("DVD/Test Game.iso")
        .expect("OPL ISO should be present");
    assert_eq!(file.size, LOGICAL_SIZE as u64);

    let mut reader = open_file(&trace.output_vfs, "DVD/Test Game.iso").unwrap();
    let mut across_hunks = [0; 20];
    let bytes_read = reader
        .read_at((HUNK_SIZE - 7) as u64, &mut across_hunks)
        .unwrap();

    assert_eq!(bytes_read, across_hunks.len());
    assert_eq!(
        across_hunks,
        std::array::from_fn(|index| fixture_byte(HUNK_SIZE - 7 + index))
    );
}

/// Builds the smallest useful uncompressed CHD v5 image for integration tests.
///
/// The layout exercises the real CHD parser, metadata iterator, hunk map, and
/// random-access reader without requiring `chdman` during test execution.
fn dvd_chd_fixture() -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"MComprHD");
    push_u32(&mut bytes, HEADER_SIZE as u32);
    push_u32(&mut bytes, 5);
    for _ in 0..4 {
        push_u32(&mut bytes, 0);
    }
    push_u64(&mut bytes, LOGICAL_SIZE as u64);
    push_u64(&mut bytes, MAP_OFFSET as u64);
    push_u64(&mut bytes, METADATA_OFFSET as u64);
    push_u32(&mut bytes, HUNK_SIZE as u32);
    push_u32(&mut bytes, SECTOR_SIZE as u32);
    bytes.resize(HEADER_SIZE, 0);

    for hunk in 0..HUNK_COUNT {
        push_u32(&mut bytes, (DATA_OFFSET / HUNK_SIZE + hunk) as u32);
    }

    bytes.extend_from_slice(b"DVD ");
    push_u32(&mut bytes, 0);
    push_u64(&mut bytes, 0);
    bytes.resize(DATA_OFFSET, 0);
    bytes.extend((0..LOGICAL_SIZE).map(fixture_byte));

    bytes
}

fn fixture_byte(offset: usize) -> u8 {
    ((offset * 17 + 3) % 251) as u8
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
