use std::path::Path;

use crate::error::RetromountError;

pub trait FilesystemAdapter {
    fn mount(self, mountpoint: &Path) -> Result<(), RetromountError>;
}
