use crate::core::game_image::GameImage;

pub trait OutputPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    /// Determine if this output format supports the given image
    fn supports(&self, image: &GameImage) -> bool;
}