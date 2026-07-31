use std::fs;
use std::io::Write;

use retromount::core::vfs_reader::open_vfs_file;
use retromount::engine::mount::prepare_mount_session;
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

fn sbi_fixture(fill: u8) -> Vec<u8> {
    let mut sbi = b"SBI\0".to_vec();
    sbi.extend_from_slice(&[0x00, 0x02, 0x00, 0x01]);
    sbi.extend_from_slice(&[fill; 10]);
    sbi
}

fn read_file(session: &retromount::mount::session::MountSession, inode: u64) -> Vec<u8> {
    let file = session.file(inode).expect("mounted file");
    let mut reader = open_vfs_file(file).unwrap();
    let mut bytes = vec![0; file.size as usize];
    let mut offset = 0;
    while offset < bytes.len() {
        let read = reader.read_at(offset as u64, &mut bytes[offset..]).unwrap();
        if read == 0 {
            break;
        }
        offset += read;
    }
    bytes.truncate(offset);
    bytes
}

#[test]
fn presents_mixed_mode_ps1_as_generated_cue_and_live_track_bins() {
    let directory = tempfile::tempdir().unwrap();
    let cue_path = directory.path().join("Game.cue");
    fs::write(
        &cue_path,
        concat!(
            "FILE \"Game.bin\" BINARY\n",
            "  TRACK 01 MODE1/2352\n",
            "    INDEX 01 00:00:00\n",
            "  TRACK 02 AUDIO\n",
            "    PREGAP 00:02:00\n",
            "    INDEX 00 00:00:01\n",
            "    INDEX 01 00:00:02\n",
        ),
    )
    .unwrap();

    let data = mode1_sector(0x31);
    let file_backed_pregap = vec![0x42; 2352];
    let audio = vec![0x53; 2352];
    let mut source = data.clone();
    source.extend_from_slice(&file_backed_pregap);
    source.extend_from_slice(&audio);
    fs::write(directory.path().join("Game.bin"), source).unwrap();
    let sbi_bytes = sbi_fixture(0x64);
    fs::write(directory.path().join("Game.sbi"), &sbi_bytes).unwrap();

    let session = prepare_mount_session(&cue_path, "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "Game")
        .expect("game artifact-set directory");
    let cue = session
        .lookup_child(game.inode, "Game (Disc 1).cue")
        .expect("generated CUE");
    let track1 = session
        .lookup_child(game.inode, "Game (Disc 1) (Track 01).bin")
        .expect("data track BIN");
    let track2 = session
        .lookup_child(game.inode, "Game (Disc 1) (Track 02).bin")
        .expect("audio track BIN");
    let sbi = session
        .lookup_child(game.inode, "Game (Disc 1).sbi")
        .expect("renamed SBI sidecar");

    assert_eq!(read_file(&session, track1.inode), data);
    let mut expected_track2 = file_backed_pregap;
    expected_track2.extend_from_slice(&audio);
    assert_eq!(read_file(&session, track2.inode), expected_track2);
    assert_eq!(read_file(&session, sbi.inode), sbi_bytes);
    assert_eq!(
        String::from_utf8(read_file(&session, cue.inode)).unwrap(),
        concat!(
            "FILE \"Game (Disc 1) (Track 01).bin\" BINARY\n",
            "  TRACK 01 MODE1/2352\n",
            "    INDEX 01 00:00:00\n",
            "FILE \"Game (Disc 1) (Track 02).bin\" BINARY\n",
            "  TRACK 02 AUDIO\n",
            "    PREGAP 00:02:00\n",
            "    INDEX 00 00:00:00\n",
            "    INDEX 01 00:00:01\n",
        )
    );
}

#[test]
fn presents_stored_zip_cue_bin_through_the_same_duckstation_path() {
    let archive = tempfile::NamedTempFile::new().unwrap();
    let sector = mode1_sector(0x67);
    let sbi_bytes = sbi_fixture(0x76);
    {
        let mut zip = zip::ZipWriter::new(archive.reopen().unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("Zip Game.cue", stored).unwrap();
        zip.write_all(
            b"FILE \"Zip Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        zip.start_file("Zip Game.bin", stored).unwrap();
        zip.write_all(&sector).unwrap();
        zip.start_file("Zip Game.sbi", stored).unwrap();
        zip.write_all(&sbi_bytes).unwrap();
        zip.finish().unwrap();
    }
    let zip_path = archive.path().with_extension("zip");
    fs::copy(archive.path(), &zip_path).unwrap();

    let session = prepare_mount_session(&zip_path, "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "Zip Game")
        .expect("stored ZIP game directory");
    let track = session
        .lookup_child(game.inode, "Zip Game (Disc 1) (Track 01).bin")
        .expect("stored ZIP track BIN");
    let sbi = session
        .lookup_child(game.inode, "Zip Game (Disc 1).sbi")
        .expect("stored ZIP SBI");

    assert_eq!(read_file(&session, track.inode), sector);
    assert_eq!(read_file(&session, sbi.inode), sbi_bytes);
}

#[test]
fn presents_a_cooked_ps1_iso_as_an_honest_mode1_2048_cue_bin_set() {
    let directory = tempfile::tempdir().unwrap();
    let iso_path = directory.path().join("Cooked Game.iso");
    let iso = (0..4096)
        .map(|offset| (offset % 251) as u8)
        .collect::<Vec<_>>();
    fs::write(&iso_path, &iso).unwrap();

    let session = prepare_mount_session(&iso_path, "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "Cooked Game")
        .expect("cooked ISO game directory");
    let cue = session
        .lookup_child(game.inode, "Cooked Game (Disc 1).cue")
        .expect("generated CUE");
    let track = session
        .lookup_child(game.inode, "Cooked Game (Disc 1) (Track 01).bin")
        .expect("generated MODE1/2048 BIN");

    assert_eq!(read_file(&session, track.inode), iso);
    assert_eq!(
        String::from_utf8(read_file(&session, cue.inode)).unwrap(),
        concat!(
            "FILE \"Cooked Game (Disc 1) (Track 01).bin\" BINARY\n",
            "  TRACK 01 MODE1/2048\n",
            "    INDEX 01 00:00:00\n",
        )
    );
}

#[test]
fn presents_a_stored_zip_cooked_iso_through_the_same_live_path() {
    let archive = tempfile::NamedTempFile::new().unwrap();
    let iso = vec![0x5c; 4096];
    {
        let mut zip = zip::ZipWriter::new(archive.reopen().unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("Zip ISO.iso", stored).unwrap();
        zip.write_all(&iso).unwrap();
        zip.finish().unwrap();
    }
    let zip_path = archive.path().with_extension("zip");
    fs::copy(archive.path(), &zip_path).unwrap();

    let session = prepare_mount_session(&zip_path, "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "Zip ISO")
        .expect("stored ZIP ISO game directory");
    let track = session
        .lookup_child(game.inode, "Zip ISO (Disc 1) (Track 01).bin")
        .expect("stored ZIP ISO track");

    assert_eq!(read_file(&session, track.inode), iso);
}

#[test]
fn rejects_cooked_ps1_iso_with_partial_sector_geometry() {
    let directory = tempfile::tempdir().unwrap();
    let iso_path = directory.path().join("Partial.iso");
    fs::write(&iso_path, vec![0; 2049]).unwrap();

    let error = prepare_mount_session(&iso_path, "duckstation").unwrap_err();

    assert!(error.to_string().contains("whole 2048-byte sectors"));
}

#[test]
fn rejects_ambiguous_case_insensitive_sbi_sidecars() {
    let archive = tempfile::NamedTempFile::new().unwrap();
    {
        let mut zip = zip::ZipWriter::new(archive.reopen().unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("Game.cue", stored).unwrap();
        zip.write_all(b"FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n")
            .unwrap();
        zip.start_file("Game.bin", stored).unwrap();
        zip.write_all(&mode1_sector(0x32)).unwrap();
        zip.start_file("Game.sbi", stored).unwrap();
        zip.write_all(&sbi_fixture(0x41)).unwrap();
        zip.start_file("game.SBI", stored).unwrap();
        zip.write_all(&sbi_fixture(0x42)).unwrap();
        zip.finish().unwrap();
    }
    let zip_path = archive.path().with_extension("zip");
    fs::copy(archive.path(), &zip_path).unwrap();

    let error = prepare_mount_session(&zip_path, "duckstation").unwrap_err();

    assert!(error.to_string().contains("multiple matching SBI"));
}

#[test]
fn rejects_malformed_sbi_sidecars() {
    let directory = tempfile::tempdir().unwrap();
    let cue_path = directory.path().join("Game.cue");
    fs::write(
        &cue_path,
        "FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n",
    )
    .unwrap();
    fs::write(directory.path().join("Game.bin"), mode1_sector(0x32)).unwrap();
    fs::write(directory.path().join("Game.sbi"), b"NOT-SBI").unwrap();

    let error = prepare_mount_session(&cue_path, "duckstation").unwrap_err();

    assert!(error.to_string().contains("SBI sidecar"));
}

#[test]
fn presents_multi_disc_ps1_as_ordered_cue_bin_sets_and_relative_m3u() {
    let directory = tempfile::tempdir().unwrap();
    for (disc, fill) in [(2, 0x72), (1, 0x61)] {
        let cue_name = format!("Multi Game (Disc {disc}).cue");
        let bin_name = format!("Multi Game (Disc {disc}).bin");
        fs::write(
            directory.path().join(&cue_name),
            format!("FILE \"{bin_name}\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n"),
        )
        .unwrap();
        fs::write(directory.path().join(bin_name), mode1_sector(fill)).unwrap();
    }

    let session = prepare_mount_session(directory.path(), "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "Multi Game")
        .expect("atomic multi-disc game directory");
    let disc1 = session
        .lookup_child(game.inode, "Multi Game (Disc 1)")
        .expect("disc 1 artifact set");
    let disc2 = session
        .lookup_child(game.inode, "Multi Game (Disc 2)")
        .expect("disc 2 artifact set");
    let playlist = session
        .lookup_child(game.inode, "Multi Game.m3u")
        .expect("multi-disc playlist");
    let disc1_track = session
        .lookup_child(disc1.inode, "Multi Game (Disc 1) (Track 01).bin")
        .expect("disc 1 track");
    let disc2_track = session
        .lookup_child(disc2.inode, "Multi Game (Disc 2) (Track 01).bin")
        .expect("disc 2 track");

    assert_eq!(read_file(&session, disc1_track.inode), mode1_sector(0x61));
    assert_eq!(read_file(&session, disc2_track.inode), mode1_sector(0x72));
    assert_eq!(
        String::from_utf8(read_file(&session, playlist.inode)).unwrap(),
        concat!(
            "Multi Game (Disc 1)/Multi Game (Disc 1).cue\n",
            "Multi Game (Disc 2)/Multi Game (Disc 2).cue\n",
        )
    );
}

#[test]
fn presents_multi_disc_chds_through_the_same_relative_m3u_contract() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("CHD Multi (Disc 2).chd"),
        cd_chd_fixture(false),
    )
    .unwrap();
    fs::write(
        directory.path().join("CHD Multi (Disc 1).chd"),
        cd_chd_fixture(false),
    )
    .unwrap();

    let session = prepare_mount_session(directory.path(), "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "CHD Multi")
        .expect("CHD multi-disc game");
    let playlist = session
        .lookup_child(game.inode, "CHD Multi.m3u")
        .expect("CHD multi-disc playlist");

    assert_eq!(
        String::from_utf8(read_file(&session, playlist.inode)).unwrap(),
        concat!(
            "CHD Multi (Disc 1)/CHD Multi (Disc 1).cue\n",
            "CHD Multi (Disc 2)/CHD Multi (Disc 2).cue\n",
        )
    );
}

#[test]
fn presents_a_mixed_mode_ps1_chd_through_the_duckstation_cue_bin_view() {
    let directory = tempfile::tempdir().unwrap();
    let chd_path = directory.path().join("CHD Game.chd");
    fs::write(&chd_path, cd_chd_fixture(false)).unwrap();
    let sbi_bytes = sbi_fixture(0x58);
    fs::write(directory.path().join("CHD Game.sbi"), &sbi_bytes).unwrap();

    let session = prepare_mount_session(&chd_path, "duckstation").unwrap();
    let ps1 = session
        .lookup_child(session.root_inode(), "PS1")
        .expect("PS1 presentation root");
    let game = session
        .lookup_child(ps1.inode, "CHD Game")
        .expect("CHD game directory");
    let track1 = session
        .lookup_child(game.inode, "CHD Game (Disc 1) (Track 01).bin")
        .expect("CHD data track");
    let track2 = session
        .lookup_child(game.inode, "CHD Game (Disc 1) (Track 02).bin")
        .expect("CHD audio track");
    let sbi = session
        .lookup_child(game.inode, "CHD Game (Disc 1).sbi")
        .expect("CHD-adjacent SBI");

    assert_eq!(
        read_file(&session, track1.inode),
        expected_chd_track_bytes(0, 4)
    );
    assert_eq!(
        read_file(&session, track2.inode),
        expected_chd_track_bytes(4, 4)
    );
    assert_eq!(read_file(&session, sbi.inode), sbi_bytes);
}

#[test]
fn rejects_chd_subchannel_data_that_cue_bin_cannot_preserve() {
    let directory = tempfile::tempdir().unwrap();
    let chd_path = directory.path().join("Protected.chd");
    fs::write(&chd_path, cd_chd_fixture(true)).unwrap();

    let error = prepare_mount_session(&chd_path, "duckstation").unwrap_err();

    assert!(error.to_string().contains("subchannel data"));
}

fn cd_chd_fixture(with_subchannel: bool) -> Vec<u8> {
    const HEADER_SIZE: usize = 124;
    const FRAME_SIZE: usize = 2448;
    const FRAMES_PER_HUNK: usize = 4;
    const HUNK_SIZE: usize = FRAME_SIZE * FRAMES_PER_HUNK;
    const HUNK_COUNT: usize = 2;
    const MAP_OFFSET: usize = HEADER_SIZE;
    const METADATA_OFFSET: usize = MAP_OFFSET + HUNK_COUNT * 4;
    const DATA_OFFSET: usize = HUNK_SIZE;

    let subtype = if with_subchannel { "RW_RAW" } else { "NONE" };
    let metadata = [
        format!(
            "TRACK:1 TYPE:MODE2_RAW SUBTYPE:{subtype} FRAMES:4 PREGAP:0 PGTYPE:MODE1 PGSUB:NONE POSTGAP:0\0"
        )
        .into_bytes(),
        b"TRACK:2 TYPE:AUDIO SUBTYPE:NONE FRAMES:4 PREGAP:1 PGTYPE:VAUDIO PGSUB:NONE POSTGAP:0\0"
            .to_vec(),
    ];
    let first_next = METADATA_OFFSET + 16 + metadata[0].len();

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MComprHD");
    push_u32(&mut bytes, HEADER_SIZE as u32);
    push_u32(&mut bytes, 5);
    for _ in 0..4 {
        push_u32(&mut bytes, 0);
    }
    push_u64(&mut bytes, (HUNK_SIZE * HUNK_COUNT) as u64);
    push_u64(&mut bytes, MAP_OFFSET as u64);
    push_u64(&mut bytes, METADATA_OFFSET as u64);
    push_u32(&mut bytes, HUNK_SIZE as u32);
    push_u32(&mut bytes, FRAME_SIZE as u32);
    bytes.resize(HEADER_SIZE, 0);

    for hunk in 0..HUNK_COUNT {
        push_u32(&mut bytes, (DATA_OFFSET / HUNK_SIZE + hunk) as u32);
    }
    push_metadata(&mut bytes, *b"CHT2", &metadata[0], first_next as u64);
    push_metadata(&mut bytes, *b"CHT2", &metadata[1], 0);
    bytes.resize(DATA_OFFSET, 0);

    for frame in 0..HUNK_COUNT * FRAMES_PER_HUNK {
        bytes.extend((0..2352).map(|offset| chd_sector_byte(frame, offset)));
        bytes.extend((0..96).map(|offset| {
            if with_subchannel && frame < 4 {
                (offset as u8).wrapping_add(1)
            } else {
                0
            }
        }));
    }
    bytes
}

fn expected_chd_track_bytes(start_frame: usize, frames: usize) -> Vec<u8> {
    (start_frame..start_frame + frames)
        .flat_map(|frame| (0..2352).map(move |offset| chd_sector_byte(frame, offset)))
        .collect()
}

fn chd_sector_byte(frame: usize, offset: usize) -> u8 {
    ((frame * 13 + offset * 7 + 5) % 251) as u8
}

fn push_metadata(bytes: &mut Vec<u8>, tag: [u8; 4], value: &[u8], next: u64) {
    bytes.extend_from_slice(&tag);
    push_u32(bytes, value.len() as u32);
    push_u64(bytes, next);
    bytes.extend_from_slice(value);
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
