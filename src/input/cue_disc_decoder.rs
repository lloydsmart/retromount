use std::io;
use std::path::Path;

use crate::core::cd::{CdDisc, CdIndex, CdSectorFormat, CdTrack, CdTrackKind};
use crate::core::content::{ContentId, DecodedContent, DecodedDiscContent, DiscMedia, LogicalDisc};
use crate::core::cue::{parse_cue, CueFileEntry, CueTrackEntry};
use crate::core::input_content::InputContent;
use crate::core::reader_handle::ReaderHandle;
use crate::core::source::{SourceObject, SourceRef};
use crate::core::source_resolver::resolve_source_ref;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;
use crate::readers::range_reader::RangeReader;
use crate::readers::raw_cd_sector_reader::{
    validate_raw_cd_track, RawCdDataMode, RawCdSectorReader,
};

#[derive(Debug, Default)]
pub struct CueDiscDecoder {
    require_opl_projection: bool,
}

impl CueDiscDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_opl() -> Self {
        Self {
            require_opl_projection: true,
        }
    }

    fn decode_cd(object: &SourceObject) -> io::Result<CdDisc> {
        let cue_text = read_cue_text(object)?;
        let files = parse_cue(&cue_text)?;
        let mut tracks = Vec::new();
        let mut previous_track_number = None;

        for file in files {
            if !file.file_type.eq_ignore_ascii_case("BINARY") {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "CUE file '{}' uses unsupported FILE type '{}'; expected BINARY",
                        file.path.display(),
                        file.file_type
                    ),
                ));
            }
            if file.tracks.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("CUE file '{}' contains no tracks", file.path.display()),
                ));
            }

            let source = object.resolve_relative(&file.path);
            let content = resolve_source_ref(&source)?;
            content.open_random_access().map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "CUE track source '{}' requires efficient random access: {error}",
                        source
                    ),
                )
            })?;

            ensure_compatible_file_track_sizes(&file)?;

            for (track_index, entry) in file.tracks.iter().enumerate() {
                if previous_track_number.is_some_and(|number| entry.number <= number) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "CUE track numbers must be strictly increasing",
                    ));
                }
                previous_track_number = Some(entry.number);

                tracks.push(build_track(
                    &file,
                    track_index,
                    entry,
                    source.clone(),
                    content.clone(),
                )?);
            }
        }

        Ok(CdDisc { tracks })
    }
}

impl InputDecoder for CueDiscDecoder {
    fn supports(&self, identity: &InputIdentity) -> bool {
        matches!(identity, InputIdentity::DiscImage)
    }

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<DecodedContent>, io::Error> {
        if !self.supports(identity) {
            return Ok(Vec::new());
        }

        let cd_disc = Self::decode_cd(object)?;
        let logical_disc = cd_disc.opl_logical_track().map(|track| LogicalDisc {
            media: DiscMedia::Cd,
            sector_size: 2048,
            sector_count: track.sector_count,
            content: track
                .logical_content
                .clone()
                .expect("OPL-compatible track must have logical content"),
        });
        if self.require_opl_projection && logical_disc.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "CUE/BIN disc contains mixed-mode, audio, Form 2, or multiple-track content that OPL cannot represent as a 2048-byte ISO",
            ));
        }
        let title = Path::new(&object.name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&object.name)
            .to_string();
        let consumed_sources = unique_track_sources(&cd_disc);

        Ok(vec![DecodedContent::Disc(DecodedDiscContent {
            id: ContentId::new(object.name.clone()),
            source: object.source.clone(),
            title,
            disc_number: 1,
            consumed_sources,
            cd_disc: Some(cd_disc),
            logical_disc,
        })])
    }
}

fn build_track(
    file: &CueFileEntry,
    track_index: usize,
    entry: &CueTrackEntry,
    source: SourceRef,
    content: InputContent,
) -> io::Result<CdTrack> {
    let (kind, sector_format) = track_format(&entry.mode)?;
    let sector_size = u64::from(sector_format.encoded_sector_size());
    let index_one = entry
        .indexes
        .iter()
        .find(|index| index.number == 1)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("CUE track {:02} has no INDEX 01", entry.number),
            )
        })?
        .sector;
    let next_start = file
        .tracks
        .get(track_index + 1)
        .and_then(track_start_sector);

    if !content.size.is_multiple_of(sector_size) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "CUE source '{}' size is not a multiple of its {sector_size}-byte sectors",
                source
            ),
        ));
    }

    let file_sectors = content.size / sector_size;
    let end_sector = next_start.unwrap_or(file_sectors);
    if index_one >= end_sector || end_sector > file_sectors {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("CUE track {:02} has an invalid source range", entry.number),
        ));
    }

    let sector_count = end_sector - index_one;
    let source_offset = index_one
        .checked_mul(sector_size)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CUE track offset overflow"))?;
    let logical_content = logical_track_handle(
        &content,
        &source,
        source_offset,
        sector_count,
        sector_format,
    )?;
    let file_backed_pregap = entry
        .indexes
        .iter()
        .find(|index| index.number == 0)
        .map(|index| {
            index_one.checked_sub(index.sector).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("CUE track {:02} INDEX 00 follows INDEX 01", entry.number),
                )
            })
        })
        .transpose()?
        .unwrap_or(0);
    let pregap_sectors = file_backed_pregap
        .checked_add(entry.pregap_sectors.unwrap_or(0))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CUE pregap overflow"))?;

    Ok(CdTrack {
        number: entry.number,
        kind,
        sector_format,
        sector_count,
        source,
        source_offset,
        encoded_content: content.handle,
        logical_content,
        indexes: entry
            .indexes
            .iter()
            .map(|index| CdIndex {
                number: index.number,
                sector: index.sector,
            })
            .collect(),
        pregap_sectors,
    })
}

fn track_start_sector(track: &CueTrackEntry) -> Option<u64> {
    track
        .indexes
        .iter()
        .find(|index| index.number == 0)
        .or_else(|| track.indexes.iter().find(|index| index.number == 1))
        .map(|index| index.sector)
}

fn ensure_compatible_file_track_sizes(file: &CueFileEntry) -> io::Result<()> {
    let mut sizes = file
        .tracks
        .iter()
        .map(|track| track_format(&track.mode).map(|(_, format)| format.encoded_sector_size()));
    let first = sizes.next().transpose()?.unwrap_or(0);

    if sizes.any(|size| size.is_ok_and(|size| size != first)) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "CUE file '{}' mixes encoded sector sizes in one source file",
                file.path.display()
            ),
        ));
    }

    Ok(())
}

fn track_format(mode: &str) -> io::Result<(CdTrackKind, CdSectorFormat)> {
    match mode.to_ascii_uppercase().as_str() {
        "MODE1/2048" => Ok((CdTrackKind::Data, CdSectorFormat::Mode1_2048)),
        "MODE1/2352" => Ok((CdTrackKind::Data, CdSectorFormat::Mode1_2352)),
        "MODE2/2352" => Ok((CdTrackKind::Data, CdSectorFormat::Mode2_2352)),
        "AUDIO" => Ok((CdTrackKind::Audio, CdSectorFormat::Audio2352)),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported CUE track mode '{mode}'"),
        )),
    }
}

fn logical_track_handle(
    content: &InputContent,
    source: &SourceRef,
    source_offset: u64,
    sector_count: u64,
    format: CdSectorFormat,
) -> io::Result<Option<ReaderHandle>> {
    let id = format!(
        "cd-logical:{}:{source_offset}:{sector_count}:{format:?}",
        source
    );
    let source_content = content.clone();

    match format {
        CdSectorFormat::Mode1_2048 => {
            let length = sector_count.checked_mul(2048).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "logical track length overflow")
            })?;
            Ok(Some(ReaderHandle::new(id, move || {
                Ok(Box::new(RangeReader::new(
                    source_content.open_random_access()?,
                    source_offset,
                    length,
                )?))
            })))
        }
        CdSectorFormat::Mode1_2352 => {
            validate_raw_cd_track(
                content.open_random_access()?,
                source_offset,
                sector_count,
                RawCdDataMode::Mode1,
            )?;
            Ok(Some(ReaderHandle::new(id, move || {
                Ok(Box::new(RawCdSectorReader::new(
                    source_content.open_random_access()?,
                    source_offset,
                    sector_count,
                    RawCdDataMode::Mode1,
                )?))
            })))
        }
        CdSectorFormat::Mode2_2352 => {
            let compatible = validate_raw_cd_track(
                content.open_random_access()?,
                source_offset,
                sector_count,
                RawCdDataMode::Mode2Form1,
            )?;
            Ok(compatible.then(|| {
                ReaderHandle::new(id, move || {
                    Ok(Box::new(RawCdSectorReader::new(
                        source_content.open_random_access()?,
                        source_offset,
                        sector_count,
                        RawCdDataMode::Mode2Form1,
                    )?))
                })
            }))
        }
        CdSectorFormat::Audio2352 => Ok(None),
    }
}

fn read_cue_text(object: &SourceObject) -> io::Result<String> {
    let mut reader = object.content.open()?;
    let size = usize::try_from(object.content.size).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "CUE file is too large to parse on this platform",
        )
    })?;
    let mut bytes = vec![0; size];
    let mut read = 0;

    while read < bytes.len() {
        let count = reader.read_at(read as u64, &mut bytes[read..])?;
        if count == 0 {
            break;
        }
        read += count;
    }
    bytes.truncate(read);

    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn unique_track_sources(disc: &CdDisc) -> Vec<SourceRef> {
    let mut sources = Vec::new();
    for track in &disc.tracks {
        if !sources.contains(&track.source) {
            sources.push(track.source.clone());
        }
    }
    sources
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use super::*;
    use crate::input::file_source::FileInputSource;
    use crate::input::source::InputSource;

    const SYNC: [u8; 12] = [
        0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
    ];

    fn object_for(path: &Path) -> SourceObject {
        FileInputSource::new(path).enumerate().unwrap().remove(0)
    }

    fn mode1_sector(fill: u8) -> Vec<u8> {
        let mut sector = vec![0; 2352];
        sector[..SYNC.len()].copy_from_slice(&SYNC);
        sector[15] = 1;
        sector[16..16 + 2048].fill(fill);
        sector
    }

    fn mode2_sector(fill: u8, form2: bool) -> Vec<u8> {
        let mut sector = vec![0; 2352];
        sector[..SYNC.len()].copy_from_slice(&SYNC);
        sector[15] = 2;
        let submode = if form2 { 0x20 } else { 0 };
        sector[16..20].copy_from_slice(&[1, 2, submode, 4]);
        sector[20..24].copy_from_slice(&[1, 2, submode, 4]);
        sector[24..24 + if form2 { 2324 } else { 2048 }].fill(fill);
        sector
    }

    #[test]
    fn decodes_a_raw_single_track_as_a_live_logical_cd() {
        let directory = tempfile::tempdir().unwrap();
        let cue_path = directory.path().join("Game.cue");
        let bin_path = directory.path().join("Game.bin");
        fs::write(
            &cue_path,
            "FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        let mut bin = fs::File::create(&bin_path).unwrap();
        bin.write_all(&mode1_sector(0x11)).unwrap();
        bin.write_all(&mode1_sector(0x22)).unwrap();

        let decoded = CueDiscDecoder::new()
            .decode(&object_for(&cue_path), &InputIdentity::DiscImage)
            .unwrap();
        let DecodedContent::Disc(disc) = &decoded[0] else {
            panic!("expected disc");
        };

        let cd = disc.cd_disc.as_ref().unwrap();
        assert_eq!(cd.tracks.len(), 1);
        assert_eq!(cd.tracks[0].sector_format, CdSectorFormat::Mode1_2352);
        assert_eq!(cd.tracks[0].sector_count, 2);
        let logical = disc.logical_disc.as_ref().unwrap();
        assert_eq!(logical.media, DiscMedia::Cd);
        let mut reader = logical.content.open().unwrap();
        let mut output = [0; 8];
        reader.read_at(2044, &mut output).unwrap();
        assert_eq!(&output, &[0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]);
    }

    #[test]
    fn preserves_mixed_mode_tracks_without_exposing_a_lossy_opl_view() {
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
                "    PREGAP 00:02:00\n",
                "    INDEX 01 00:00:00\n",
            ),
        )
        .unwrap();
        fs::write(directory.path().join("data.bin"), vec![0x44; 4096]).unwrap();
        fs::write(directory.path().join("audio.bin"), vec![0x55; 2352]).unwrap();

        let decoded = CueDiscDecoder::new()
            .decode(&object_for(&cue_path), &InputIdentity::DiscImage)
            .unwrap();
        let DecodedContent::Disc(disc) = &decoded[0] else {
            panic!("expected disc");
        };
        let cd = disc.cd_disc.as_ref().unwrap();

        assert_eq!(cd.tracks.len(), 2);
        assert_eq!(cd.tracks[1].kind, CdTrackKind::Audio);
        assert_eq!(cd.tracks[1].pregap_sectors, 150);
        assert!(disc.logical_disc.is_none());
    }

    #[test]
    fn maps_mode2_form1_and_preserves_form2_without_an_iso_projection() {
        for (form2, expected_projection) in [(false, true), (true, false)] {
            let directory = tempfile::tempdir().unwrap();
            let cue_path = directory.path().join("Mode2.cue");
            fs::write(
                &cue_path,
                "FILE \"Mode2.bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n",
            )
            .unwrap();
            fs::write(
                directory.path().join("Mode2.bin"),
                mode2_sector(0x63, form2),
            )
            .unwrap();

            let decoded = CueDiscDecoder::new()
                .decode(&object_for(&cue_path), &InputIdentity::DiscImage)
                .unwrap();
            let DecodedContent::Disc(disc) = &decoded[0] else {
                panic!("expected disc");
            };

            assert_eq!(disc.logical_disc.is_some(), expected_projection);
            assert_eq!(
                disc.cd_disc.as_ref().unwrap().tracks[0]
                    .logical_content
                    .is_some(),
                expected_projection
            );
        }
    }

    #[test]
    fn derives_track_ranges_and_file_backed_pregaps_in_a_single_bin() {
        let directory = tempfile::tempdir().unwrap();
        let cue_path = directory.path().join("Mixed.cue");
        fs::write(
            &cue_path,
            concat!(
                "FILE \"Mixed.bin\" BINARY\n",
                "  TRACK 01 MODE1/2352\n",
                "    INDEX 01 00:00:00\n",
                "  TRACK 02 AUDIO\n",
                "    INDEX 00 00:00:01\n",
                "    INDEX 01 00:00:02\n",
            ),
        )
        .unwrap();
        let mut encoded = mode1_sector(0x41);
        encoded.extend(vec![0x52; 2352]);
        encoded.extend(vec![0x63; 2352]);
        fs::write(directory.path().join("Mixed.bin"), encoded).unwrap();

        let decoded = CueDiscDecoder::new()
            .decode(&object_for(&cue_path), &InputIdentity::DiscImage)
            .unwrap();
        let DecodedContent::Disc(disc) = &decoded[0] else {
            panic!("expected disc");
        };
        let tracks = &disc.cd_disc.as_ref().unwrap().tracks;

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].sector_count, 1);
        assert_eq!(tracks[1].source_offset, 2 * 2352);
        assert_eq!(tracks[1].sector_count, 1);
        assert_eq!(tracks[1].pregap_sectors, 1);
        assert_eq!(tracks[0].source, tracks[1].source);
        assert!(disc.logical_disc.is_none());
    }
}
