use std::collections::HashMap;
use std::io;

use crate::core::reader_handle::ReaderHandle;
use crate::core::source::SourceRef;
use crate::core::vfs::VfsFile;
use crate::output::capabilities::EncoderCapability;
use crate::output::plan::{ArtifactId, ArtifactRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterializedArtifact {
    SourceBacked { source: SourceRef, size: u64 },
    Inline(Vec<u8>),
    ReaderBacked { handle: ReaderHandle, size: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedNamedArtifact {
    pub name: String,
    pub artifact: MaterializedArtifact,
}

impl MaterializedNamedArtifact {
    pub fn new(name: impl Into<String>, artifact: MaterializedArtifact) -> Self {
        Self {
            name: name.into(),
            artifact,
        }
    }
}

impl MaterializedArtifact {
    pub fn to_vfs_file(&self, name: &str) -> VfsFile {
        match self {
            Self::SourceBacked { source, size } => {
                VfsFile::source_backed(name, *size, source.clone())
            }
            Self::Inline(contents) => VfsFile::inline(name, contents.clone()),
            Self::ReaderBacked { handle, size } => {
                VfsFile::reader_backed(name, *size, handle.clone())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaterializationContext {
    pub artifact_names: HashMap<ArtifactId, String>,
}

pub trait OutputEncoder: Send + Sync {
    fn plugin_id(&self) -> &str;

    fn capabilities(&self) -> Vec<EncoderCapability>;

    fn materialize(
        &self,
        file_name: &str,
        artifact: &ArtifactRequest,
        selected_capability_id: &str,
        context: &MaterializationContext,
    ) -> Result<MaterializedArtifact, io::Error>;

    fn materialize_set(
        &self,
        _artifact_name: &str,
        _artifact: &ArtifactRequest,
        selected_capability_id: &str,
        _context: &MaterializationContext,
    ) -> Result<Vec<MaterializedNamedArtifact>, io::Error> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "encoder '{}' capability '{}' does not materialize artifact sets",
                self.plugin_id(),
                selected_capability_id
            ),
        ))
    }
}
