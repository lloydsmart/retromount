use crate::core::disc::Disc;
use crate::core::platform::Platform;

#[derive(Debug, Clone)]
pub struct GameImage {
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub discs: Vec<Disc>,
}