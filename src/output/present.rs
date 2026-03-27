use crate::core::content::NormalizedContent;
use crate::core::vfs::VfsDirectory;

pub trait OutputPresenter: Send + Sync {
    fn present(&self, content: &[NormalizedContent]) -> VfsDirectory;
}
