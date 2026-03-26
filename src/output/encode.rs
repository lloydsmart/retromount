use crate::core::content::{Content, GameContent, GamePart};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFile {
    pub name: String,
    pub size: u64,
}

pub trait OutputEncoder {
    fn can_encode(&self, content: &Content) -> bool;

    fn encode(&self, content: &Content) -> Result<EncodedFile, std::io::Error>;

    /// Encode an individual game part (ROM or Disc)
    fn encode_game_part(
        &self,
        game: &GameContent,
        part: &GamePart,
    ) -> Result<EncodedFile, std::io::Error>;

    /// Encode a playlist file for a multi-disc game
    fn encode_playlist(
        &self,
        game: &GameContent,
        entries: &[String],
    ) -> Result<EncodedFile, std::io::Error>;
}
