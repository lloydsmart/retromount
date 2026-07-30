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
    pub sector_count: u64,
    pub source: SourceRef,
    pub source_offset: u64,
    pub encoded_content: ReaderHandle,
    pub logical_content: Option<ReaderHandle>,
    pub indexes: Vec<CdIndex>,
    pub pregap_sectors: u64,
}

impl CdTrack {
    pub fn encoded_len(&self) -> Option<u64> {
        u64::from(self.sector_format.encoded_sector_size()).checked_mul(self.sector_count)
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
            pregap_sectors: 0,
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
}
