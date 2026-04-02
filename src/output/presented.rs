use crate::core::content::{GameContent, NormalizedContent};
use crate::output::basic_encoder::BasicEncoder;
use crate::output::encode::EncodedFile;
use crate::output::encode::OutputEncoder;
use crate::policy::PolicySet;

#[derive(Debug, Clone)]
pub enum PresentedEntry {
    Game(PresentedGame),
    File(EncodedFile),
}

#[derive(Debug, Clone)]
pub struct PresentedGame {
    pub game: GameContent,
}

pub fn build_presented_entries(
    content: &[NormalizedContent],
    encoder: &BasicEncoder,
    policy: &PolicySet,
) -> Vec<PresentedEntry> {
    content
        .iter()
        .filter_map(|item| match item {
            NormalizedContent::Game(game) => {
                Some(PresentedEntry::Game(PresentedGame { game: game.clone() }))
            }
            NormalizedContent::Bytes(_) | NormalizedContent::Text(_) => {
                if !encoder.can_encode(item) {
                    return None;
                }

                encoder.encode(item, policy).ok().map(PresentedEntry::File)
            }
        })
        .collect()
}
