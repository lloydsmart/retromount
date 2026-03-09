#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackType {
    Data,
    Audio,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub number: u32,
    pub kind: TrackType,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_data_track() {
        let track = Track {
            number: 1,
            kind: TrackType::Data,
            size: 1024,
        };

        assert_eq!(track.number, 1);
        assert!(matches!(track.kind, TrackType::Data));
        assert_eq!(track.size, 1024);
    }
}