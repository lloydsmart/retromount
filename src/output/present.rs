use crate::core::content::Content;
use crate::core::vfs::VfsDirectory;

pub trait OutputPresenter: Send + Sync {
    fn present(&self, content: &[Content]) -> VfsDirectory;
}
