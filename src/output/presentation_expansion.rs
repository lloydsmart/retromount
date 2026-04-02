use crate::core::content::{DiscPart, GameContent, GamePart, NormalizedContent};
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::policy::PolicySet;

/// Returns true when a game consists entirely of multiple disc parts.
pub fn is_multi_disc_game(game: &GameContent) -> bool {
    game.parts.len() > 1
        && game
            .parts
            .iter()
            .all(|part| matches!(part, GamePart::Disc(_)))
}

/// Returns disc parts sorted by disc number.
pub fn sorted_disc_parts(game: &GameContent) -> Vec<&DiscPart> {
    let mut parts: Vec<&DiscPart> = game
        .parts
        .iter()
        .filter_map(|part| match part {
            GamePart::Disc(disc) => Some(disc),
            _ => None,
        })
        .collect();

    parts.sort_by(|a, b| a.disc_number.cmp(&b.disc_number));
    parts
}

/// Encodes a game that expands to a single output file.
pub fn encode_game(
    encoder: &impl OutputEncoder,
    game: &GameContent,
    policy: &PolicySet,
) -> EncodedFile {
    encoder
        .encode(&NormalizedContent::Game(game.clone()), policy)
        .unwrap_or_else(|err| panic!("game should encode for '{}': {err}", game.title))
}

/// Encodes a disc file for a multi-disc game.
pub fn encode_disc(
    encoder: &impl OutputEncoder,
    game: &GameContent,
    disc: &DiscPart,
    policy: &PolicySet,
) -> EncodedFile {
    encoder
        .encode_game_part(game, &GamePart::Disc(disc.clone()), policy)
        .unwrap_or_else(|err| {
            panic!(
                "multi-disc game part should encode for '{}' disc {}: {err}",
                game.title, disc.disc_number
            )
        })
}

/// Encodes a playlist file for a multi-disc game.
pub fn encode_playlist(
    encoder: &impl OutputEncoder,
    game: &GameContent,
    disc_names: &[String],
    policy: &PolicySet,
) -> EncodedFile {
    encoder
        .encode_playlist(game, disc_names, policy)
        .unwrap_or_else(|err| {
            panic!(
                "multi-disc playlist should encode for '{}': {err}",
                game.title
            )
        })
}
