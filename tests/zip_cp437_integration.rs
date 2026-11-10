use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;

use retromount::core::content::DecodedContent;
use retromount::core::input_content::InputAccess;
use retromount::core::reader::Reader;
use retromount::core::source::{SourceOrigin, SourceRef};
use retromount::core::source_resolver::{open_source_ref, resolve_source_ref, zip_entry_content};
use retromount::input::cue_disc_decoder::CueDiscDecoder;
use retromount::input::decode::InputDecoder;
use retromount::input::identify::InputIdentity;
use retromount::input::source::InputSource;
use retromount::input::zip_source::ZipInputSource;
use retromount::readers::zip_reader::ZipReader;

// Build ZIP records independently: ZipWriter would normally encode Unicode
// names as UTF-8, so it cannot prove the legacy filename regression.
struct Entry<'a> {
    name: &'a [u8],
    utf8: bool,
    deflate: bool,
    data: &'a [u8],
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn u16le(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, n: u32) {
    out.extend(n.to_le_bytes());
}

fn archive(entries: &[Entry<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let offset = out.len() as u32;
        let flags = if entry.utf8 { 1 << 11 } else { 0 };
        let method = if entry.deflate { 8 } else { 0 };
        let len = u16::try_from(entry.data.len()).unwrap();
        // A valid raw DEFLATE stream with one final uncompressed block.
        let mut payload = Vec::new();
        if entry.deflate {
            payload.push(1);
            u16le(&mut payload, len);
            u16le(&mut payload, !len);
        }
        payload.extend(entry.data);
        let crc = crc32(entry.data);
        u32le(&mut out, 0x04034b50);
        for n in [20, flags, method, 0, 0] {
            u16le(&mut out, n);
        }
        for n in [crc, payload.len() as u32, entry.data.len() as u32] {
            u32le(&mut out, n);
        }
        u16le(&mut out, entry.name.len() as u16);
        u16le(&mut out, 0);
        out.extend(entry.name);
        out.extend(payload);

        u32le(&mut central, 0x02014b50);
        for n in [20, 20, flags, method, 0, 0] {
            u16le(&mut central, n);
        }
        for n in [
            crc,
            (len as u32) + if entry.deflate { 5 } else { 0 },
            len as u32,
        ] {
            u32le(&mut central, n);
        }
        for n in [entry.name.len() as u16, 0, 0, 0, 0] {
            u16le(&mut central, n);
        }
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend(entry.name);
    }
    let offset = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    u32le(&mut out, 0x06054b50);
    for n in [0, 0, entries.len() as u16, entries.len() as u16] {
        u16le(&mut out, n);
    }
    u32le(&mut out, size);
    u32le(&mut out, offset);
    u16le(&mut out, 0);
    out
}

fn source(path: &Path, name: &str) -> SourceRef {
    SourceRef::new(format!("zip:{}#{name}", path.to_string_lossy()))
}

fn assert_reads(reader: &mut dyn Reader, expected: &[u8]) {
    assert_eq!(reader.len(), expected.len() as u64);
    // Out-of-order reads force the Deflate reader to reopen, including after EOF.
    for (offset, count) in [
        (5, 7),
        (0, expected.len()),
        (3, 4),
        (expected.len(), 1),
        (0, 3),
    ] {
        let mut bytes = vec![0; count];
        let mut read = 0;
        while read < count {
            let n = reader
                .read_at((offset + read) as u64, &mut bytes[read..])
                .unwrap();
            if n == 0 {
                break;
            }
            read += n;
        }
        let end = (offset + count).min(expected.len());
        assert_eq!(&bytes[..read], &expected[offset..end]);
        assert_eq!(read, end - offset);
    }
}

#[test]
fn cp437_and_control_names_round_trip_through_every_public_read_path() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("names.zip");
    let data = b"0123456789abcdefghijklmnopqrstuvwxyz";
    for deflate in [false, true] {
        let entries = [
            Entry {
                name: b"caf\x82.rom",
                utf8: false,
                deflate,
                data,
            },
            Entry {
                name: b"plain.rom",
                utf8: false,
                deflate,
                data: b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            },
            Entry {
                name: "日本語.rom".as_bytes(),
                utf8: true,
                deflate,
                data: b"abcdefghijklmnopqrstuvwxyz0123456789",
            },
        ];
        let bytes = archive(&entries);
        // Check raw names, flags, methods and payload independently of Retromount.
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        for (index, expected) in entries.iter().enumerate() {
            let mut file = zip.by_index(index).unwrap();
            assert_eq!(file.name_raw(), expected.name);
            assert_eq!(
                file.compression(),
                if deflate {
                    zip::CompressionMethod::Deflated
                } else {
                    zip::CompressionMethod::Stored
                }
            );
            let mut payload = Vec::new();
            file.read_to_end(&mut payload).unwrap();
            assert_eq!(payload, expected.data);
        }
        assert_eq!(&bytes[6..8], &[0, 0]);
        fs::write(&path, bytes).unwrap();
        let objects = ZipInputSource::new(&path).enumerate().unwrap();
        assert_eq!(objects.len(), 3);
        for (name, expected) in [
            ("café.rom", entries[0].data),
            ("plain.rom", entries[1].data),
            ("日本語.rom", entries[2].data),
        ] {
            let object = objects.iter().find(|o| o.name == name).unwrap();
            assert_eq!(object.source, source(&path, name));
            assert_eq!(
                object.origin,
                SourceOrigin::ZipEntry {
                    archive_path: path.clone(),
                    entry_name: name.to_owned()
                }
            );
            assert_eq!(
                object.content.access,
                if deflate {
                    InputAccess::Sequential
                } else {
                    InputAccess::RandomAccess
                }
            );
            assert_reads(object.content.open().unwrap().as_mut(), expected);
            assert_reads(&mut ZipReader::open(&path, name).unwrap(), expected);
            assert_reads(open_source_ref(&object.source).unwrap().as_mut(), expected);
            let resolved = resolve_source_ref(&object.source).unwrap();
            assert_eq!(resolved.size, expected.len() as u64);
            assert_eq!(resolved.access, object.content.access);
            assert_reads(resolved.open().unwrap().as_mut(), expected);
        }
    }
}

#[test]
fn cp437_cue_resolves_bin_and_reads_sbi_in_a_cp437_directory() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("disc.zip");
    let cue = "FILE \"pisté.bin\" BINARY\n TRACK 01 MODE1/2048\n INDEX 01 00:00:00\n";
    let bin: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
    let sbi = [
        b'S', b'B', b'I', 0, 0, 2, 0, 1, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    ];
    for deflate in [false, true] {
        fs::write(
            &path,
            archive(&[
                Entry {
                    name: b"donn\x82es/caf\x82.cue",
                    utf8: false,
                    deflate,
                    data: cue.as_bytes(),
                },
                // BIN needs efficient random access; compression is deliberately stored.
                Entry {
                    name: b"donn\x82es/pist\x82.bin",
                    utf8: false,
                    deflate: false,
                    data: &bin,
                },
                Entry {
                    name: b"donn\x82es/CAF\x82.SBI",
                    utf8: false,
                    deflate,
                    data: &sbi,
                },
                Entry {
                    name: b"other/caf\x82.sbi",
                    utf8: false,
                    deflate,
                    data: b"not the sidecar",
                },
            ]),
        )
        .unwrap();
        let objects = ZipInputSource::new(&path).enumerate().unwrap();
        let object = objects.iter().find(|o| o.name.ends_with(".cue")).unwrap();
        let decoded = CueDiscDecoder::new()
            .decode(object, &InputIdentity::DiscImage)
            .unwrap();
        let DecodedContent::Disc(disc) = &decoded[0] else {
            panic!("expected disc")
        };
        let cd = disc.cd_disc.as_ref().unwrap();
        assert_eq!(cd.tracks.len(), 1);
        assert_eq!(cd.tracks[0].sector_count, 2);
        assert_eq!(cd.tracks[0].source, source(&path, "données/pisté.bin"));
        assert_reads(cd.tracks[0].encoded_content.open().unwrap().as_mut(), &bin);
        assert_reads(
            disc.logical_disc
                .as_ref()
                .unwrap()
                .content
                .open()
                .unwrap()
                .as_mut(),
            &bin,
        );
        let sidecar = cd.sbi.as_ref().unwrap();
        assert_eq!(sidecar.source, source(&path, "données/CAFé.SBI"));
        assert_eq!(sidecar.size, sbi.len() as u64);
        assert_reads(sidecar.content.open().unwrap().as_mut(), &sbi);
        assert!(disc.consumed_sources.contains(&sidecar.source));
    }
}

fn assert_invalid<T>(result: io::Result<T>) {
    let error = result.err().expect("ambiguous decoded name must fail");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("ambiguous"));
}

#[test]
fn decoded_name_collisions_are_rejected_in_both_archive_orders() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("collision.zip");
    for deflate in [false, true] {
        for reverse in [false, true] {
            let mut entries = [
                Entry {
                    name: b"caf\x82.rom",
                    utf8: false,
                    deflate,
                    data: b"legacy member contents",
                },
                Entry {
                    name: "café.rom".as_bytes(),
                    utf8: true,
                    deflate,
                    data: b"different UTF8 contents",
                },
            ];
            if reverse {
                entries.reverse();
            }
            fs::write(&path, archive(&entries)).unwrap();
            assert_invalid(ZipInputSource::new(&path).enumerate());
            assert_invalid(ZipReader::open(&path, "café.rom"));
            assert_invalid(open_source_ref(&source(&path, "café.rom")));
            assert_invalid(resolve_source_ref(&source(&path, "café.rom")));
            assert_invalid(
                zip_entry_content(&path, "café.rom", 22, InputAccess::Sequential).open(),
            );
        }
    }
}

#[test]
fn missing_names_remain_not_found() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("missing.zip");
    fs::write(
        &path,
        archive(&[Entry {
            name: b"caf\x82.rom",
            utf8: false,
            deflate: false,
            data: b"contents",
        }]),
    )
    .unwrap();
    for name in ["missing.rom", "Café.rom", "cafe\u{301}.rom"] {
        assert_eq!(
            ZipReader::open(&path, name).err().unwrap().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            resolve_source_ref(&source(&path, name))
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
}

#[test]
fn unflagged_utf8_names_also_participate_in_decoded_collisions() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("unflagged.zip");
    for deflate in [false, true] {
        let unflagged = Entry {
            name: "café.rom".as_bytes(),
            utf8: false,
            deflate,
            data: b"unflagged UTF8 payload",
        };
        // ZIP 9.4.1 recognises valid UTF-8 even when bit 11 is clear.
        fs::write(&path, archive(std::slice::from_ref(&unflagged))).unwrap();
        let objects = ZipInputSource::new(&path).enumerate().unwrap();
        assert_eq!(objects[0].name, "café.rom");
        assert_reads(objects[0].content.open().unwrap().as_mut(), unflagged.data);
        let entries = [
            Entry {
                name: b"caf\x82.rom",
                utf8: false,
                deflate,
                data: b"legacy member contents",
            },
            unflagged,
        ];
        fs::write(&path, archive(&entries)).unwrap();
        assert_invalid(ZipInputSource::new(&path).enumerate());
        assert_invalid(ZipReader::open(&path, "café.rom"));
        assert_invalid(open_source_ref(&source(&path, "café.rom")));
        assert_invalid(resolve_source_ref(&source(&path, "café.rom")));
    }
}

#[test]
fn compressed_reopening_rechecks_decoded_name_ambiguity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("reopened.zip");
    let legacy = Entry {
        name: b"caf\x82.rom",
        utf8: false,
        deflate: true,
        data: b"expected legacy payload",
    };
    fs::write(&path, archive(std::slice::from_ref(&legacy))).unwrap();
    let mut reader = ZipReader::open(&path, "café.rom").unwrap();
    fs::write(
        &path,
        archive(&[
            legacy,
            Entry {
                name: "café.rom".as_bytes(),
                utf8: true,
                deflate: true,
                data: b"different UTF8 payload",
            },
        ]),
    )
    .unwrap();
    assert_invalid(reader.read_at(0, &mut [0; 8]));
}
