use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackType {
    Data,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackSource {
    File(PathBuf),
    OffsetFile {
        path: PathBuf,
        offset: u64,
        length: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub number: u32,
    pub kind: TrackType,
    pub size: u64,
    pub sector_size: u32,
    pub source: TrackSource,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn creates_data_track() {
        let track = Track {
            number: 1,
            kind: TrackType::Data,
            size: 1024,
            sector_size: 2048,
            source: TrackSource::File(PathBuf::from("game.iso")),
        };

        assert_eq!(track.number, 1);
        assert!(matches!(track.kind, TrackType::Data));
        assert_eq!(track.size, 1024);
        assert_eq!(track.sector_size, 2048);
        assert!(matches!(track.source, TrackSource::File(_)));
    }
}
