use crate::core::disc::Disc;
use crate::core::platform::Platform;

#[derive(Debug, Clone)]
pub struct GameImage {
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub discs: Vec<Disc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::track::{Track, TrackType};

    #[test]
    fn creates_multi_disc_game_image() {
        let disc1 = Disc {
            number: 1,
            tracks: vec![Track {
                number: 1,
                kind: TrackType::Data,
                size: 2048,
            }],
        };

        let disc2 = Disc {
            number: 2,
            tracks: vec![Track {
                number: 1,
                kind: TrackType::Data,
                size: 2048,
            }],
        };

        let game = GameImage {
            id: "ff7".to_string(),
            title: "Final Fantasy VII".to_string(),
            platform: Platform::PlayStation,
            discs: vec![disc1, disc2],
        };

        assert_eq!(game.id, "ff7");
        assert_eq!(game.title, "Final Fantasy VII");
        assert_eq!(game.platform, Platform::PlayStation);
        assert_eq!(game.discs.len(), 2);
    }
}