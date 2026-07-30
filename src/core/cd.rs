use serde::Serialize;

use crate::core::reader_handle::ReaderHandle;
use crate::core::source::SourceRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CdTrackKind {
    Data,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CdSectorFormat {
    Mode1_2048,
    Mode1_2352,
    Mode2_2352,
    Audio2352,
}

impl CdSectorFormat {
    pub fn encoded_sector_size(self) -> u32 {
        match self {
            Self::Mode1_2048 => 2048,
            Self::Mode1_2352 | Self::Mode2_2352 | Self::Audio2352 => 2352,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CdIndex {
    pub number: u8,
    pub sector: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CdTrack {
    pub number: u8,
    pub kind: CdTrackKind,
    pub sector_format: CdSectorFormat,
    /// Number of encoded sectors in the preserved track extent. The extent
    /// begins at `source_offset` and includes file-backed INDEX 00 content.
    pub sector_count: u64,
    pub source: SourceRef,
    /// Byte offset of the earliest preserved index in the encoded source.
    pub source_offset: u64,
    pub encoded_content: ReaderHandle,
    pub logical_content: Option<ReaderHandle>,
    /// Index positions relative to the beginning of the preserved extent.
    pub indexes: Vec<CdIndex>,
    /// File-backed sectors between INDEX 00 and INDEX 01.
    pub file_backed_pregap_sectors: u64,
    /// Synthetic pregap declared with the CUE PREGAP directive.
    pub declared_pregap_sectors: u64,
}

impl CdTrack {
    pub fn encoded_len(&self) -> Option<u64> {
        u64::from(self.sector_format.encoded_sector_size()).checked_mul(self.sector_count)
    }

    pub fn index_one_sector(&self) -> Option<u64> {
        self.indexes
            .iter()
            .find(|index| index.number == 1)
            .map(|index| index.sector)
    }

    pub fn playable_sector_count(&self) -> Option<u64> {
        self.sector_count.checked_sub(self.index_one_sector()?)
    }

    pub fn total_pregap_sectors(&self) -> Option<u64> {
        self.file_backed_pregap_sectors
            .checked_add(self.declared_pregap_sectors)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CdDisc {
    pub tracks: Vec<CdTrack>,
}

impl CdDisc {
    pub fn opl_logical_track(&self) -> Option<&CdTrack> {
        let [track] = self.tracks.as_slice() else {
            return None;
        };

        (track.kind == CdTrackKind::Data && track.logical_content.is_some()).then_some(track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::inline_reader::InlineReader;

    fn handle(id: &str) -> ReaderHandle {
        ReaderHandle::new(id.to_string(), || {
            Ok(Box::new(InlineReader::new(Vec::new())))
        })
    }

    #[test]
    fn exposes_only_single_logical_data_tracks_to_opl() {
        let data = CdTrack {
            number: 1,
            kind: CdTrackKind::Data,
            sector_format: CdSectorFormat::Mode1_2352,
            sector_count: 10,
            source: SourceRef::new("file:game.bin"),
            source_offset: 0,
            encoded_content: handle("encoded"),
            logical_content: Some(handle("logical")),
            indexes: vec![CdIndex {
                number: 1,
                sector: 0,
            }],
            file_backed_pregap_sectors: 0,
            declared_pregap_sectors: 0,
        };
        let audio = CdTrack {
            number: 2,
            kind: CdTrackKind::Audio,
            sector_format: CdSectorFormat::Audio2352,
            logical_content: None,
            ..data.clone()
        };

        assert!(CdDisc {
            tracks: vec![data.clone()]
        }
        .opl_logical_track()
        .is_some());
        assert!(CdDisc {
            tracks: vec![data.clone(), audio]
        }
        .opl_logical_track()
        .is_none());
        assert!(CdDisc { tracks: vec![] }.opl_logical_track().is_none());
    }

    #[test]
    fn reports_playable_and_pregap_sector_counts() {
        let track = CdTrack {
            number: 2,
            kind: CdTrackKind::Audio,
            sector_format: CdSectorFormat::Audio2352,
            sector_count: 12,
            source: SourceRef::new("file:game.bin"),
            source_offset: 2352,
            encoded_content: handle("encoded"),
            logical_content: None,
            indexes: vec![
                CdIndex {
                    number: 0,
                    sector: 0,
                },
                CdIndex {
                    number: 1,
                    sector: 2,
                },
            ],
            file_backed_pregap_sectors: 2,
            declared_pregap_sectors: 3,
        };

        assert_eq!(track.playable_sector_count(), Some(10));
        assert_eq!(track.total_pregap_sectors(), Some(5));
        assert_eq!(track.encoded_len(), Some(12 * 2352));
    }
}
