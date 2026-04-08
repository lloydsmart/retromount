use crate::core::content::NormalizedContent;
use crate::core::vfs::VfsDirectory;
use crate::policy::PolicySet;

/// Defines how normalized content is exposed in the output VFS.
///
/// Presenters are responsible for output structure, grouping, and layout.
/// Representation-specific decisions such as filenames and file backing
/// remain encoder responsibilities.
pub trait OutputPresenter: Send + Sync {
    fn present(&self, content: &[NormalizedContent], policy: &PolicySet) -> VfsDirectory;
}
