use std::io;

use chd::metadata::{KnownMetadata, Metadata, MetadataTag};
use chd::{Chd, Error as ChdError};

use crate::core::cd::{CdDisc, CdIndex, CdSectorFormat, CdSubchannelFormat, CdTrack, CdTrackKind};
use crate::core::content::{ContentId, DecodedContent, DecodedDiscContent, DiscMedia, LogicalDisc};
use crate::core::input_content::InputContent;
use crate::core::reader_cursor::ReaderCursor;
use crate::core::reader_handle::ReaderHandle;
use crate::core::source::SourceObject;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentity;
use crate::readers::chd_reader::ChdReader;
use crate::readers::interleaved_sector_reader::InterleavedSectorReader;

const DVD_METADATA_TAG: u32 = u32::from_be_bytes(*b"DVD ");
const DVD_SECTOR_SIZE: u32 = 2048;
const CD_FRAME_SIZE: u32 = 2448;
const CD_DATA_SIZE: u32 = 2352;
const CD_SUBCHANNEL_SIZE: u32 = 96;
const CD_TRACK_PADDING: u64 = 4;

#[derive(Debug, Default)]
pub struct ChdDiscDecoder;

#[derive(Debug, Clone)]
struct ChdDiscInfo {
    has_parent: bool,
    has_dvd_metadata: bool,
    unit_bytes: u32,
    logical_bytes: u64,
    cd_tracks: Vec<ChdTrackInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChdTrackInfo {
    number: u8,
    track_type: String,
    subtype: String,
    frames: u64,
    pregap: u64,
    pregap_in_data: bool,
    postgap: u64,
}

impl ChdDiscDecoder {
    pub fn new() -> Self {
        Self
    }

    fn inspect(content: &InputContent) -> io::Result<ChdDiscInfo> {
        let cursor = ReaderCursor::new(content.open_random_access()?);
        let mut chd = Chd::open(cursor, None).map_err(map_chd_open_error)?;
        let header = chd.header();
        let has_parent = header.has_parent();
        let unit_bytes = header.unit_bytes();
        let logical_bytes = header.logical_bytes();
        let metadata = Vec::<Metadata>::try_from(chd.metadata_refs())
            .map_err(|error| map_chd_metadata_error(error, "failed to read CHD metadata"))?;
        let has_dvd_metadata = metadata
            .iter()
            .any(|metadata| metadata.metatag() == DVD_METADATA_TAG);
        let cd_tracks = metadata
            .iter()
            .filter(|metadata| {
                matches!(
                    metadata.metatag(),
                    tag if tag == KnownMetadata::CdRomTrack.metatag()
                        || tag == KnownMetadata::CdRomTrack2.metatag()
                )
            })
            .map(parse_cd_track_metadata)
            .collect::<io::Result<Vec<_>>>()?;

        Ok(ChdDiscInfo {
            has_parent,
            has_dvd_metadata,
            unit_bytes,
            logical_bytes,
            cd_tracks,
        })
    }

    fn logical_disc(content: &InputContent, info: ChdDiscInfo) -> io::Result<LogicalDisc> {
        if info.has_parent {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "parent/delta CHDs are not supported",
            ));
        }
        if !info.has_dvd_metadata {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "CHD does not describe DVD media",
            ));
        }
        if info.unit_bytes != DVD_SECTOR_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "DVD CHD unit size must be {DVD_SECTOR_SIZE} bytes, found {}",
                    info.unit_bytes
                ),
            ));
        }
        if info.logical_bytes == 0 || !info.logical_bytes.is_multiple_of(DVD_SECTOR_SIZE.into()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DVD CHD logical size must contain whole 2048-byte sectors",
            ));
        }

        let handle = content.handle.clone();
        Ok(LogicalDisc {
            media: DiscMedia::Dvd,
            sector_size: DVD_SECTOR_SIZE,
            sector_count: info.logical_bytes / u64::from(DVD_SECTOR_SIZE),
            content: crate::core::reader_handle::ReaderHandle::new(
                format!("chd:{}", content.handle.id()),
                move || Ok(Box::new(ChdReader::open(&handle)?)),
            ),
        })
    }

    fn cd_disc(
        content: &InputContent,
        source: &crate::core::source::SourceRef,
        info: &ChdDiscInfo,
    ) -> io::Result<CdDisc> {
        if info.has_parent {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "parent/delta CHDs are not supported",
            ));
        }
        if info.cd_tracks.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "CHD does not describe CD media",
            ));
        }
        if info.unit_bytes != CD_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "CD CHD unit size must be {CD_FRAME_SIZE} bytes, found {}",
                    info.unit_bytes
                ),
            ));
        }

        let mut chd_frame_offset = 0_u64;
        let mut tracks = Vec::with_capacity(info.cd_tracks.len());
        let mut previous_track_number = None;
        for track in &info.cd_tracks {
            if previous_track_number.is_some_and(|number| track.number <= number) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "CHD CD track numbers must be strictly increasing",
                ));
            }
            previous_track_number = Some(track.number);
            if track.postgap != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "CHD track {:02} has a postgap that the canonical CD model cannot preserve",
                        track.number
                    ),
                ));
            }
            let (kind, sector_format, data_size) = chd_track_format(&track.track_type)?;
            let subchannel_format = chd_subchannel_format(&track.subtype)?;
            let (extent_offset, sector_count, file_backed_pregap, declared_pregap) =
                if track.pregap_in_data {
                    (chd_frame_offset, track.frames, track.pregap, 0)
                } else {
                    (
                        chd_frame_offset,
                        track.frames.checked_sub(track.pregap).ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "CHD track {:02} pregap exceeds its frame count",
                                    track.number
                                ),
                            )
                        })?,
                        0,
                        track.pregap,
                    )
                };
            if sector_count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("CHD track {:02} has no encoded sectors", track.number),
                ));
            }

            let encoded_content = chd_sector_handle(
                content,
                extent_offset,
                sector_count,
                0,
                data_size,
                format!("track-{:02}", track.number),
            );
            let subchannel_content = subchannel_format.map(|_| {
                chd_sector_handle(
                    content,
                    extent_offset,
                    sector_count,
                    CD_DATA_SIZE,
                    CD_SUBCHANNEL_SIZE,
                    format!("track-{:02}-subchannel", track.number),
                )
            });
            let indexes = if file_backed_pregap > 0 {
                vec![
                    CdIndex {
                        number: 0,
                        sector: 0,
                    },
                    CdIndex {
                        number: 1,
                        sector: file_backed_pregap,
                    },
                ]
            } else {
                vec![CdIndex {
                    number: 1,
                    sector: 0,
                }]
            };
            tracks.push(CdTrack {
                number: track.number,
                kind,
                sector_format,
                sector_count,
                source: source.clone(),
                source_offset: 0,
                encoded_content,
                logical_content: None,
                subchannel_content,
                subchannel_format,
                indexes,
                file_backed_pregap_sectors: file_backed_pregap,
                declared_pregap_sectors: declared_pregap,
            });

            chd_frame_offset = chd_frame_offset
                .checked_add(round_up(track.frames, CD_TRACK_PADDING)?)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "CHD track offset overflow")
                })?;
        }

        let required_bytes = chd_frame_offset
            .checked_mul(u64::from(CD_FRAME_SIZE))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CHD CD size overflow"))?;
        if required_bytes > info.logical_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CHD CD track metadata exceeds its logical content",
            ));
        }

        Ok(CdDisc { tracks })
    }
}

fn map_chd_metadata_error(error: ChdError, context: &str) -> io::Error {
    let mapped = map_chd_open_error(error);
    io::Error::new(mapped.kind(), format!("{context}: {mapped}"))
}

fn parse_cd_track_metadata(metadata: &Metadata) -> io::Result<ChdTrackInfo> {
    let text = std::str::from_utf8(&metadata.value)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "CHD CD metadata is not UTF-8"))?
        .trim_end_matches('\0');
    let mut values = std::collections::HashMap::new();
    for field in text.split_whitespace() {
        let Some((key, value)) = field.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("malformed CHD CD metadata field '{field}'"),
            ));
        };
        values.insert(key, value);
    }
    let parse_number = |key: &str| -> io::Result<u64> {
        values
            .get(key)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("CHD CD metadata is missing {key}"),
                )
            })?
            .parse()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("CHD CD metadata has invalid {key}"),
                )
            })
    };
    let number = u8::try_from(parse_number("TRACK")?).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "CHD CD track number is out of range",
        )
    })?;
    let frames = parse_number("FRAMES")?;
    if number == 0 || frames == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "CHD CD track number and frame count must be non-zero",
        ));
    }
    let pregap = values
        .get("PREGAP")
        .map(|_| parse_number("PREGAP"))
        .transpose()?
        .unwrap_or(0);
    let postgap = values
        .get("POSTGAP")
        .map(|_| parse_number("POSTGAP"))
        .transpose()?
        .unwrap_or(0);
    let pregap_type = values.get("PGTYPE").copied().unwrap_or("NONE");

    Ok(ChdTrackInfo {
        number,
        track_type: values
            .get("TYPE")
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "CHD CD metadata is missing TYPE",
                )
            })?
            .to_string(),
        subtype: values.get("SUBTYPE").copied().unwrap_or("NONE").to_string(),
        frames,
        pregap,
        pregap_in_data: pregap > 0 && pregap_type.starts_with('V'),
        postgap,
    })
}

fn chd_track_format(track_type: &str) -> io::Result<(CdTrackKind, CdSectorFormat, u32)> {
    match track_type {
        "MODE1" => Ok((CdTrackKind::Data, CdSectorFormat::Mode1_2048, 2048)),
        "MODE1_RAW" => Ok((CdTrackKind::Data, CdSectorFormat::Mode1_2352, 2352)),
        "MODE2_RAW" => Ok((CdTrackKind::Data, CdSectorFormat::Mode2_2352, 2352)),
        "AUDIO" => Ok((CdTrackKind::Audio, CdSectorFormat::Audio2352, 2352)),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported CHD CD track type '{track_type}'"),
        )),
    }
}

fn chd_subchannel_format(subtype: &str) -> io::Result<Option<CdSubchannelFormat>> {
    match subtype {
        "NONE" => Ok(None),
        "RW" => Ok(Some(CdSubchannelFormat::Normal)),
        "RW_RAW" => Ok(Some(CdSubchannelFormat::Raw)),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported CHD CD subchannel type '{subtype}'"),
        )),
    }
}

fn chd_sector_handle(
    content: &InputContent,
    frame_offset: u64,
    sector_count: u64,
    output_offset: u32,
    output_size: u32,
    suffix: String,
) -> ReaderHandle {
    let source = content.handle.clone();
    ReaderHandle::new(format!("chd:{}:{suffix}", content.handle.id()), move || {
        Ok(Box::new(InterleavedSectorReader::new(
            Box::new(ChdReader::open(&source)?),
            frame_offset * u64::from(CD_FRAME_SIZE),
            sector_count,
            u64::from(CD_FRAME_SIZE),
            u64::from(output_offset),
            u64::from(output_size),
        )?))
    })
}

fn round_up(value: u64, multiple: u64) -> io::Result<u64> {
    value
        .checked_add(multiple - 1)
        .map(|value| value / multiple * multiple)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CHD track padding overflow"))
}

fn map_chd_open_error(error: ChdError) -> io::Error {
    let kind = match error {
        ChdError::InvalidFile
        | ChdError::InvalidData
        | ChdError::InvalidMetadata
        | ChdError::InvalidMetadataSize
        | ChdError::ReadError => io::ErrorKind::InvalidData,
        ChdError::RequiresParent
        | ChdError::NotSupported
        | ChdError::UnsupportedFormat
        | ChdError::UnsupportedVersion => io::ErrorKind::Unsupported,
        _ => io::ErrorKind::Other,
    };

    io::Error::new(kind, format!("failed to open CHD: {error}"))
}

impl InputDecoder for ChdDiscDecoder {
    fn supports(&self, identity: &InputIdentity) -> bool {
        matches!(identity, InputIdentity::ChdDisc)
    }

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<DecodedContent>, io::Error> {
        if !self.supports(identity) {
            return Ok(Vec::new());
        }

        let info = Self::inspect(&object.content)?;
        let (cd_disc, logical_disc) = if info.has_dvd_metadata {
            (None, Some(Self::logical_disc(&object.content, info)?))
        } else if !info.cd_tracks.is_empty() {
            (
                Some(Self::cd_disc(&object.content, &object.source, &info)?),
                None,
            )
        } else {
            (None, Some(Self::logical_disc(&object.content, info)?))
        };
        let title = std::path::Path::new(&object.name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&object.name)
            .to_string();

        Ok(vec![DecodedContent::Disc(DecodedDiscContent {
            id: ContentId::new(object.name.clone()),
            source: object.source.clone(),
            title,
            disc_number: 1,
            consumed_sources: Vec::new(),
            cd_disc,
            logical_disc,
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input_content::{InputAccess, InputContent};
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::SourceRef;
    use crate::readers::inline_reader::InlineReader;

    fn test_content() -> InputContent {
        InputContent::new(
            1,
            InputAccess::RandomAccess,
            ReaderHandle::new("test:chd", || Ok(Box::new(InlineReader::new(vec![0])))),
        )
    }

    fn valid_info() -> ChdDiscInfo {
        ChdDiscInfo {
            has_parent: false,
            has_dvd_metadata: true,
            unit_bytes: DVD_SECTOR_SIZE,
            logical_bytes: u64::from(DVD_SECTOR_SIZE) * 3,
            cd_tracks: Vec::new(),
        }
    }

    #[test]
    fn builds_a_live_logical_dvd_from_valid_geometry() {
        let disc = ChdDiscDecoder::logical_disc(&test_content(), valid_info()).unwrap();

        assert_eq!(disc.media, DiscMedia::Dvd);
        assert_eq!(disc.sector_size, DVD_SECTOR_SIZE);
        assert_eq!(disc.sector_count, 3);
        assert_eq!(disc.byte_len(), Some(6144));
        assert_eq!(disc.content.id(), "chd:test:chd");
    }

    #[test]
    fn rejects_parent_and_non_dvd_chds() {
        let parent = ChdDiscInfo {
            has_parent: true,
            ..valid_info()
        };
        let non_dvd = ChdDiscInfo {
            has_dvd_metadata: false,
            ..valid_info()
        };

        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), parent)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), non_dvd)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_non_iso_dvd_geometry() {
        let wrong_unit = ChdDiscInfo {
            unit_bytes: 4096,
            ..valid_info()
        };
        let partial_sector = ChdDiscInfo {
            logical_bytes: 2049,
            ..valid_info()
        };

        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), wrong_unit)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            ChdDiscDecoder::logical_disc(&test_content(), partial_sector)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn maps_invalid_and_unsupported_chd_errors_to_actionable_io_kinds() {
        assert_eq!(
            map_chd_open_error(ChdError::InvalidData).kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            map_chd_open_error(ChdError::UnsupportedVersion).kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            map_chd_open_error(ChdError::RequiresParent).kind(),
            io::ErrorKind::Unsupported
        );
    }

    #[test]
    fn parses_modern_cd_track_metadata_and_valid_pregaps() {
        let metadata = Metadata {
            metatag: KnownMetadata::CdRomTrack2.metatag(),
            value: b"TRACK:2 TYPE:AUDIO SUBTYPE:RW_RAW FRAMES:452 PREGAP:150 PGTYPE:VAUDIO PGSUB:NONE POSTGAP:0\0".to_vec(),
            flags: 0,
            index: 1,
            length: 0,
        };

        let track = parse_cd_track_metadata(&metadata).unwrap();

        assert_eq!(track.number, 2);
        assert_eq!(track.track_type, "AUDIO");
        assert_eq!(track.subtype, "RW_RAW");
        assert_eq!(track.frames, 452);
        assert_eq!(track.pregap, 150);
        assert!(track.pregap_in_data);
    }

    #[test]
    fn maps_cd_tracks_and_skips_internal_track_padding() {
        let info = ChdDiscInfo {
            has_parent: false,
            has_dvd_metadata: false,
            unit_bytes: CD_FRAME_SIZE,
            logical_bytes: u64::from(CD_FRAME_SIZE) * 12,
            cd_tracks: vec![
                ChdTrackInfo {
                    number: 1,
                    track_type: "MODE2_RAW".to_string(),
                    subtype: "NONE".to_string(),
                    frames: 5,
                    pregap: 0,
                    pregap_in_data: false,
                    postgap: 0,
                },
                ChdTrackInfo {
                    number: 2,
                    track_type: "AUDIO".to_string(),
                    subtype: "RW_RAW".to_string(),
                    frames: 4,
                    pregap: 1,
                    pregap_in_data: true,
                    postgap: 0,
                },
            ],
        };

        let disc =
            ChdDiscDecoder::cd_disc(&test_content(), &SourceRef::new("file:game.chd"), &info)
                .unwrap();

        assert_eq!(disc.tracks.len(), 2);
        assert_eq!(disc.tracks[0].sector_count, 5);
        assert_eq!(disc.tracks[1].sector_count, 4);
        assert_eq!(disc.tracks[1].file_backed_pregap_sectors, 1);
        assert_eq!(disc.tracks[1].index_one_sector(), Some(1));
        assert_eq!(
            disc.tracks[1].subchannel_format,
            Some(CdSubchannelFormat::Raw)
        );
        assert!(disc.tracks[1].subchannel_content.is_some());
        assert!(disc.tracks[1].encoded_content.id().contains("track-02"));
    }
}
