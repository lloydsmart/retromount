use crate::core::content::NormalizedContent;
use crate::core::vfs::VfsDirectory;

/// Defines how normalized content is exposed in the output VFS.
///
/// Presenters are responsible for output structure, grouping, and layout.
/// In the default implementation, a presenter may compose an encoder while
/// constructing the final VFS tree, but representation-specific decisions
/// such as filenames and file backing remain encoder responsibilities.
pub trait OutputPresenter: Send + Sync {
    fn present(&self, content: &[NormalizedContent]) -> VfsDirectory;
}
