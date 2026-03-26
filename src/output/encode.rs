use crate::core::content::{Content, GameContent, GamePart};
use crate::core::source::SourceRef;
use crate::core::vfs::VfsFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodedBacking {
    SourceBacked { size: u64, source: SourceRef },
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
            EncodedBacking::SourceBacked { size, source } => {
                VfsFile::source_backed(self.name.clone(), *size, source.clone())
            }
            EncodedBacking::Inline(bytes) => VfsFile::inline(self.name.clone(), bytes.clone()),
        }
    }

    pub fn size(&self) -> u64 {
        match &self.backing {
            EncodedBacking::SourceBacked { size, .. } => *size,
            EncodedBacking::Inline(bytes) => bytes.len() as u64,
        }
    }
}

pub trait OutputEncoder {
    fn can_encode(&self, content: &Content) -> bool;

    fn encode(&self, content: &Content) -> Result<EncodedFile, std::io::Error>;

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
