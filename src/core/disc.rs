use crate::core::track::Track;

#[derive(Debug, Clone)]
pub struct Disc {
    pub number: u32,
    pub tracks: Vec<Track>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::track::{Track, TrackType};

    #[test]
    fn creates_disc_with_tracks() {
        let disc = Disc {
            number: 1,
            tracks: vec![
                Track {
                    number: 1,
                    kind: TrackType::Data,
                    size: 2048,
                },
                Track {
                    number: 2,
                    kind: TrackType::Audio,
                    size: 4096,
                },
            ],
        };

        assert_eq!(disc.number, 1);
        assert_eq!(disc.tracks.len(), 2);
    }
}