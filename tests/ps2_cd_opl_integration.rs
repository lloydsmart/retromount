use std::fs;
use std::io::Write;

use retromount::core::content::DiscMedia;
use retromount::core::vfs_reader::open_vfs_file;
use retromount::engine::components::pipeline_components_with_media;
use retromount::engine::mount::prepare_mount_session;
use retromount::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use retromount::engine::preview::build_input_source;
use retromount::mount::session::MountNodeKind;
use zip::write::SimpleFileOptions;

const SYNC: [u8; 12] = [
    0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
];

fn mode1_sector(fill: u8) -> Vec<u8> {
    let mut sector = vec![0; 2352];
    sector[..SYNC.len()].copy_from_slice(&SYNC);
    sector[15] = 1;
    sector[16..16 + 2048].fill(fill);
    sector
}

#[test]
fn presents_a_raw_cue_bin_ps2_cd_as_a_live_opl_iso() {
    let directory = tempfile::tempdir().unwrap();
    let cue_path = directory.path().join("Game.cue");
    fs::write(
        &cue_path,
        "FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n",
    )
    .unwrap();
    let mut bin = fs::File::create(directory.path().join("Game.bin")).unwrap();
    bin.write_all(&mode1_sector(0x31)).unwrap();
    bin.write_all(&mode1_sector(0x42)).unwrap();

    let session = prepare_mount_session(&cue_path, "opl").unwrap();
    let cd = session
        .lookup_child(session.root_inode(), "CD")
        .expect("OPL CD directory");
    let iso = session
        .lookup_child(cd.inode, "Game.iso")
        .expect("OPL CD output");
    let MountNodeKind::File { file } = &iso.kind else {
        panic!("OPL CD output should be a file");
    };
    let mut reader = open_vfs_file(file).unwrap();
    let mut output = [0; 8];

    assert_eq!(file.size, 4096);
    assert_eq!(reader.read_at(2044, &mut output).unwrap(), 8);
    assert_eq!(&output, &[0x31, 0x31, 0x31, 0x31, 0x42, 0x42, 0x42, 0x42]);
    let dvd = session
        .lookup_child(session.root_inode(), "DVD")
        .expect("OPL DVD directory");
    assert!(session.lookup_child(dvd.inode, "Game.iso").is_none());
}

#[test]
fn presents_an_explicit_cd_iso_below_the_opl_cd_directory() {
    let directory = tempfile::tempdir().unwrap();
    let iso_path = directory.path().join("Game.iso");
    fs::write(&iso_path, vec![0x6c; 4096]).unwrap();
    let source = build_input_source(&iso_path).unwrap();
    let components = pipeline_components_with_media("opl", Some(DiscMedia::Cd)).unwrap();

    let trace = run_pipeline_with_presentation_options(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &PipelineOptions {
            normalization: components.normalization,
            plugin_registry: None,
        },
    )
    .unwrap();

    let file = trace
        .output_vfs
        .find_file("CD/Game.iso")
        .expect("OPL CD ISO output");
    assert_eq!(file.size, 4096);
    assert!(trace
        .output_vfs
        .find_directory("DVD")
        .is_none_or(|directory| directory.children().is_empty()));
}

#[test]
fn rejects_mixed_mode_cds_that_opl_cannot_represent() {
    let directory = tempfile::tempdir().unwrap();
    let cue_path = directory.path().join("Mixed.cue");
    fs::write(
        &cue_path,
        concat!(
            "FILE \"data.bin\" BINARY\n",
            "  TRACK 01 MODE1/2048\n",
            "    INDEX 01 00:00:00\n",
            "FILE \"audio.bin\" BINARY\n",
            "  TRACK 02 AUDIO\n",
            "    INDEX 01 00:00:00\n",
        ),
    )
    .unwrap();
    fs::write(directory.path().join("data.bin"), vec![0x11; 4096]).unwrap();
    fs::write(directory.path().join("audio.bin"), vec![0x22; 2352]).unwrap();
    let source = build_input_source(&cue_path).unwrap();
    let components = pipeline_components_with_media("opl", None).unwrap();

    let error = run_pipeline_with_presentation_options(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &PipelineOptions {
            normalization: components.normalization,
            plugin_registry: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("OPL cannot represent"));
}

#[test]
fn presents_a_stored_zip_cue_bin_through_the_same_live_cd_path() {
    let archive = tempfile::NamedTempFile::new().unwrap();
    {
        let mut zip = zip::ZipWriter::new(archive.reopen().unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("Game.cue", stored).unwrap();
        zip.write_all(b"FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n")
            .unwrap();
        zip.start_file("Game.bin", stored).unwrap();
        zip.write_all(&mode1_sector(0x71)).unwrap();
        zip.finish().unwrap();
    }
    let zip_path = archive.path().with_extension("zip");
    fs::copy(archive.path(), &zip_path).unwrap();
    let source = build_input_source(&zip_path).unwrap();
    let components = pipeline_components_with_media("opl", None).unwrap();

    let trace = run_pipeline_with_presentation_options(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &PipelineOptions {
            normalization: components.normalization,
            plugin_registry: None,
        },
    )
    .unwrap();

    let file = trace
        .output_vfs
        .find_file("CD/Game.iso")
        .expect("stored ZIP CUE/BIN output");
    let mut reader = open_vfs_file(file).unwrap();
    let mut output = [0; 4];
    assert_eq!(reader.read_at(17, &mut output).unwrap(), 4);
    assert_eq!(output, [0x71; 4]);
}

#[test]
fn rejects_a_compressed_zip_bin_that_cannot_supply_random_access() {
    let archive = tempfile::NamedTempFile::new().unwrap();
    {
        let mut zip = zip::ZipWriter::new(archive.reopen().unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("Game.cue", stored).unwrap();
        zip.write_all(b"FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n")
            .unwrap();
        zip.start_file("Game.bin", deflated).unwrap();
        zip.write_all(&mode1_sector(0x71)).unwrap();
        zip.finish().unwrap();
    }
    let zip_path = archive.path().with_extension("zip");
    fs::copy(archive.path(), &zip_path).unwrap();
    let source = build_input_source(&zip_path).unwrap();
    let components = pipeline_components_with_media("opl", None).unwrap();

    let error = run_pipeline_with_presentation_options(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        &PipelineOptions {
            normalization: components.normalization,
            plugin_registry: None,
        },
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("efficient random access"));
}
