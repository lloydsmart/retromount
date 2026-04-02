use crate::core::content::{DiscPart, GameContent, GamePart, NormalizedContent};
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::policy::PolicySet;

pub fn is_multi_disc_game(game: &GameContent) -> bool {
    game.parts.len() > 1
        && game
            .parts
            .iter()
            .all(|part| matches!(part, GamePart::Disc(_)))
}

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

pub fn encode_game(
    encoder: &impl OutputEncoder,
    game: &GameContent,
    policy: &PolicySet,
) -> EncodedFile {
    encoder
        .encode(&NormalizedContent::Game(game.clone()), policy)
        .unwrap_or_else(|err| panic!("game should encode for '{}': {err}", game.title))
}

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

pub fn resolve_name(proposed: &str, existing: &[String], policy: &PolicySet) -> String {
    policy.conflict().resolve_name_conflict(proposed, existing)
}
