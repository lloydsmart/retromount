use std::path::Path;

use crate::core::game_image::GameImage;
use crate::error::RetromountError;

pub trait InputPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    /// Determine if this plugin can handle the provided path
    fn detect(&self, path: &Path) -> bool;

    /// Load the image and convert it into the canonical GameImage model
    fn load(&self, path: &Path) -> Result<GameImage, RetromountError>;
}
