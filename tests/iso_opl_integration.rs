use std::fs;

use retromount::core::content::{DecodedContent, DiscMedia, GamePart, NormalizedContent, Platform};
use retromount::core::normalizer::NormalizationOptions;
use retromount::core::vfs_resolver::open_file;
use retromount::engine::components::pipeline_components;
use retromount::engine::mount::prepare_mount_session;
use retromount::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use retromount::input::file_source::FileInputSource;
use retromount::mount::session::MountNodeKind;

const SECTOR_SIZE: usize = 2048;
const SECTOR_COUNT: usize = 4;
const LOGICAL_SIZE: usize = SECTOR_SIZE * SECTOR_COUNT;

#[test]
fn presents_a_filesystem_iso_through_the_live_opl_dvd_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let iso_path = temp_dir.path().join("Test Game.iso");
    fs::write(&iso_path, iso_fixture()).unwrap();

    let trace = run_opl(&iso_path).unwrap();

    assert!(trace.objects[0].supported);
    let DecodedContent::Disc(decoded_disc) = &trace.objects[0].decoded[0] else {
        panic!("ISO should decode as a disc");
    };
    let logical_disc = decoded_disc
        .logical_disc
        .as_ref()
        .expect("decoded ISO should retain live content");
    assert_eq!(logical_disc.media, DiscMedia::Dvd);
    assert_eq!(logical_disc.sector_size, SECTOR_SIZE as u32);
    assert_eq!(logical_disc.sector_count, SECTOR_COUNT as u64);

    let NormalizedContent::Game(game) = &trace.normalized[0] else {
        panic!("decoded ISO should normalize as a game");
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
    assert_read(&mut *reader, 3, 19);
    assert_read(&mut *reader, SECTOR_SIZE - 7, 23);
    assert_read(&mut *reader, LOGICAL_SIZE - 5, 5);

    let mut eof = [0; 8];
    assert_eq!(reader.read_at(LOGICAL_SIZE as u64, &mut eof).unwrap(), 0);
}

#[test]
fn prepares_a_mount_session_for_a_filesystem_iso() {
    let temp_dir = tempfile::tempdir().unwrap();
    let iso_path = temp_dir.path().join("Test Game.iso");
    fs::write(&iso_path, iso_fixture()).unwrap();

    let session = prepare_mount_session(&iso_path, "opl").unwrap();
    let dvd = session
        .lookup_child(session.root_inode(), "DVD")
        .expect("mount root should contain the OPL DVD directory");
    let iso = session
        .lookup_child(dvd.inode, "Test Game.iso")
        .expect("DVD directory should contain the live ISO");
    let MountNodeKind::File { file } = &iso.kind else {
        panic!("OPL entry should be a file");
    };

    assert_eq!(file.size, LOGICAL_SIZE as u64);
    let mut reader = retromount::core::vfs_reader::open_vfs_file(file).unwrap();
    assert_read(&mut *reader, SECTOR_SIZE + 11, 31);
}

#[test]
fn rejects_invalid_iso_geometry() {
    let temp_dir = tempfile::tempdir().unwrap();

    let empty_path = temp_dir.path().join("Empty.iso");
    fs::write(&empty_path, []).unwrap();
    let empty_error = run_opl(&empty_path).unwrap_err();
    assert_eq!(empty_error.kind(), std::io::ErrorKind::InvalidData);
    assert!(empty_error.to_string().contains("must not be empty"));

    let partial_path = temp_dir.path().join("Partial.iso");
    fs::write(&partial_path, vec![0; SECTOR_SIZE + 1]).unwrap();
    let partial_error = run_opl(&partial_path).unwrap_err();
    assert_eq!(partial_error.kind(), std::io::ErrorKind::InvalidData);
    assert!(partial_error
        .to_string()
        .contains("whole 2048-byte sectors"));
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

fn iso_fixture() -> Vec<u8> {
    (0..LOGICAL_SIZE).map(fixture_byte).collect()
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
    ((offset * 29 + 7) % 251) as u8
}
