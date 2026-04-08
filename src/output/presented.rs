use crate::core::content::{GameContent, NormalizedContent};
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::output::presentation_expansion::{
    encode_disc, encode_game, encode_playlist, is_multi_disc_game, sorted_disc_parts,
};
use crate::policy::PolicySet;

/// Expanded output intent consumed by presenters.
///
/// Presented entries represent the output artifacts that should exist
/// before layout-specific presentation is applied.
#[derive(Debug, Clone)]
pub enum PresentedEntry {
    Game(PresentedGame),
    File(EncodedFile),
}

/// A logical game plus the output files it expands into.
///
/// Presenters use the game for layout decisions and the files for placement.
#[derive(Debug, Clone)]
pub struct PresentedGame {
    pub game: GameContent,
    pub files: Vec<EncodedFile>,
}

/// Builds expanded presentation entries from normalized content.
///
/// This is the boundary where logical content is expanded into the output
/// artifacts that presenters will place into the VFS.
pub fn build_presented_entries(
    content: &[NormalizedContent],
    encoder: &dyn OutputEncoder,
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
    encoder: &dyn OutputEncoder,
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
    encoder: &dyn OutputEncoder,
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
