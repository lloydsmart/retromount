use std::fs;

use retromount::core::content::{DecodedContent, GamePart, NormalizedContent, Platform};
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
    let DecodedContent::Disc(decoded_disc) = &trace.objects[0].decoded[0] else {
        panic!("CHD should decode as a disc");
    };
    let logical_disc = decoded_disc
        .logical_disc
        .as_ref()
        .expect("decoded disc should retain live content");
    assert_eq!(logical_disc.sector_size, SECTOR_SIZE as u32);
    assert_eq!(
        logical_disc.sector_count,
        (LOGICAL_SIZE / SECTOR_SIZE) as u64
    );

    let NormalizedContent::Game(game) = &trace.normalized[0] else {
        panic!("decoded disc should normalize as a game");
    };
    assert_eq!(game.platform, Platform::Ps2);
    assert!(matches!(
        &game.parts[0],
        GamePart::Disc(part) if part.logical_disc.as_ref() == Some(logical_disc)
    ));

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

    assert_read(&mut *reader, 3, 13);
    assert_read(&mut *reader, 3, 13);
    assert_read(&mut *reader, HUNK_SIZE + 101, 31);
    assert_read(&mut *reader, 27, 19);

    let mut tail = [0; 16];
    assert_eq!(
        reader
            .read_at((LOGICAL_SIZE - 5) as u64, &mut tail)
            .unwrap(),
        5
    );
    assert_eq!(
        &tail[..5],
        &(LOGICAL_SIZE - 5..LOGICAL_SIZE)
            .map(fixture_byte)
            .collect::<Vec<_>>()
    );
    assert_eq!(reader.read_at(LOGICAL_SIZE as u64, &mut tail).unwrap(), 0);
    assert_eq!(
        reader
            .read_at((LOGICAL_SIZE + 100) as u64, &mut tail)
            .unwrap(),
        0
    );
}

#[test]
fn rejects_non_dvd_and_parent_chds_through_the_real_parser() {
    let temp_dir = tempfile::tempdir().unwrap();

    let non_dvd_path = temp_dir.path().join("Not DVD.chd");
    fs::write(&non_dvd_path, chd_fixture(*b"GDDD", false)).unwrap();
    let non_dvd_error = run_opl(&non_dvd_path).unwrap_err();
    assert_eq!(non_dvd_error.kind(), std::io::ErrorKind::Unsupported);
    assert!(non_dvd_error.to_string().contains("DVD media"));

    let parent_path = temp_dir.path().join("Parent.chd");
    fs::write(&parent_path, chd_fixture(*b"DVD ", true)).unwrap();
    let parent_error = run_opl(&parent_path).unwrap_err();
    assert_eq!(parent_error.kind(), std::io::ErrorKind::Unsupported);
    assert!(parent_error.to_string().contains("parent/delta"));
}

#[test]
fn reports_a_malformed_chd_as_invalid_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("Broken.chd");
    fs::write(&path, b"not a CHD").unwrap();

    let error = run_opl(&path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("failed to open CHD"));
}

/// Builds the smallest useful uncompressed CHD v5 image for integration tests.
///
/// The layout exercises the real CHD parser, metadata iterator, hunk map, and
/// random-access reader without requiring `chdman` during test execution.
fn dvd_chd_fixture() -> Vec<u8> {
    chd_fixture(*b"DVD ", false)
}

fn chd_fixture(metadata_tag: [u8; 4], has_parent: bool) -> Vec<u8> {
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
    if has_parent {
        bytes[104] = 1;
    }

    for hunk in 0..HUNK_COUNT {
        push_u32(&mut bytes, (DATA_OFFSET / HUNK_SIZE + hunk) as u32);
    }

    bytes.extend_from_slice(&metadata_tag);
    push_u32(&mut bytes, 0);
    push_u64(&mut bytes, 0);
    bytes.resize(DATA_OFFSET, 0);
    bytes.extend((0..LOGICAL_SIZE).map(fixture_byte));

    bytes
}

fn run_opl(path: &std::path::Path) -> std::io::Result<retromount::engine::pipeline::PipelineTrace> {
    let components = pipeline_components("opl").unwrap();
    let source = FileInputSource::new(path);
    let options = PipelineOptions {
        normalization: NormalizationOptions {
            platform_hint: Some(Platform::Ps2),
        },
        plugin_registry: None,
    };

    run_pipeline_with_presentation_options(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &options,
    )
}

fn assert_read(reader: &mut dyn retromount::core::reader::Reader, offset: usize, length: usize) {
    let mut bytes = vec![0; length];
    assert_eq!(reader.read_at(offset as u64, &mut bytes).unwrap(), length);
    assert_eq!(
        bytes,
        (offset..offset + length)
            .map(fixture_byte)
            .collect::<Vec<_>>()
    );
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
