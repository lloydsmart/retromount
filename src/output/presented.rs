use crate::core::content::{GameContent, NormalizedContent};
use crate::output::basic_encoder::BasicEncoder;
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::output::presenter_utils::{
    encode_disc, encode_game, encode_playlist, is_multi_disc_game, sorted_disc_parts,
};
use crate::policy::PolicySet;

#[derive(Debug, Clone)]
pub enum PresentedEntry {
    Game(PresentedGame),
    File(EncodedFile),
}

#[derive(Debug, Clone)]
pub struct PresentedGame {
    pub game: GameContent,
    pub files: Vec<EncodedFile>,
}

pub fn build_presented_entries(
    content: &[NormalizedContent],
    encoder: &BasicEncoder,
    policy: &PolicySet,
) -> Vec<PresentedEntry> {
    content
        .iter()
        .filter_map(|item| match item {
            NormalizedContent::Game(game) => Some(PresentedEntry::Game(build_presented_game(
                game, encoder, policy,
            ))),
            NormalizedContent::Bytes(_) | NormalizedContent::Text(_) => {
                encoder.encode(item, policy).ok().map(PresentedEntry::File)
            }
        })
        .collect()
}

fn build_presented_game(
    game: &GameContent,
    encoder: &BasicEncoder,
    policy: &PolicySet,
) -> PresentedGame {
    let files = if is_multi_disc_game(game) {
        build_multi_disc_files(game, encoder, policy)
    } else {
        vec![encode_game(encoder, game, policy)]
    };

    PresentedGame {
        game: game.clone(),
        files,
    }
}

fn build_multi_disc_files(
    game: &GameContent,
    encoder: &BasicEncoder,
    policy: &PolicySet,
) -> Vec<EncodedFile> {
    let disc_parts = sorted_disc_parts(game);
    let mut files = Vec::new();
    let mut disc_names = Vec::new();

    for disc in disc_parts {
        let encoded = encode_disc(encoder, game, disc, policy);
        disc_names.push(encoded.name.clone());
        files.push(encoded);
    }

    files.push(encode_playlist(encoder, game, &disc_names, policy));
    files
}
