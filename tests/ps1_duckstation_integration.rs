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

    assert_eq!(read_file(&session, track1.inode), data);
    let mut expected_track2 = file_backed_pregap;
    expected_track2.extend_from_slice(&audio);
    assert_eq!(read_file(&session, track2.inode), expected_track2);
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

    assert_eq!(read_file(&session, track.inode), sector);
}
