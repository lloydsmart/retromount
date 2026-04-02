use crate::core::content::{GameContent, GamePart, NormalizedContent};
use crate::core::source::SourceRef;
use crate::core::vfs::VfsFile;
use crate::policy::PolicySet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodedBacking {
    SourceBacked { source: SourceRef, size: u64 },
    Inline(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFile {
    pub name: String,
    pub backing: EncodedBacking,
}

impl EncodedFile {
    pub fn to_vfs_file(&self) -> VfsFile {
        match &self.backing {
            EncodedBacking::SourceBacked { source, size } => {
                VfsFile::source_backed(&self.name, *size, source.clone())
            }
            EncodedBacking::Inline(contents) => VfsFile::inline(&self.name, contents.clone()),
        }
    }
}

pub trait OutputEncoder: Send + Sync {
    fn can_encode(&self, content: &NormalizedContent) -> bool;

    fn encode(
        &self,
        content: &NormalizedContent,
        policy: &PolicySet,
    ) -> Result<EncodedFile, std::io::Error>;

    fn encode_game_part(
        &self,
        game: &GameContent,
        part: &GamePart,
        policy: &PolicySet,
    ) -> Result<EncodedFile, std::io::Error>;

    fn encode_playlist(
        &self,
        game: &GameContent,
        entries: &[String],
        policy: &PolicySet,
    ) -> Result<EncodedFile, std::io::Error>;
}
