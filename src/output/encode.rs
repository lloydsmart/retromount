use crate::core::content::{GameContent, GamePart, NormalizedContent};
use crate::core::source::SourceRef;
use crate::core::vfs::{FileBacking, VfsFile};

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
        let backing = match &self.backing {
            EncodedBacking::SourceBacked { source, size } => FileBacking::SourceBacked {
                source: source.clone(),
                size: *size,
            },
            EncodedBacking::Inline(contents) => FileBacking::Inline(contents.clone()),
        };

        VfsFile::with_backing(&self.name, backing)
    }
}

pub trait OutputEncoder: Send + Sync {
    fn can_encode(&self, content: &NormalizedContent) -> bool;

    fn encode(&self, content: &NormalizedContent) -> Result<EncodedFile, std::io::Error>;

    fn encode_game_part(
        &self,
        game: &GameContent,
        part: &GamePart,
    ) -> Result<EncodedFile, std::io::Error>;

    fn encode_playlist(
        &self,
        game: &GameContent,
        entries: &[String],
    ) -> Result<EncodedFile, std::io::Error>;
}
