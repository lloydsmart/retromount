use crate::core::content::NormalizedContent;
use crate::core::vfs::VfsDirectory;

/// Defines how normalized content is exposed in the output VFS.
///
/// Presenters are responsible for output structure, grouping, and layout.
/// Representation-specific decisions such as filenames and file backing
/// remain encoder responsibilities.
pub trait OutputPresenter: Send + Sync {
    fn present(&self, content: &[NormalizedContent]) -> VfsDirectory;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenterKind {
    Grouped,
    Flat,
}

impl PresenterKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "grouped" => Some(Self::Grouped),
            "flat" => Some(Self::Flat),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grouped => "grouped",
            Self::Flat => "flat",
        }
    }
}
