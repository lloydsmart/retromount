use crate::core::track::Track;

#[derive(Debug, Clone)]
pub struct Disc {
    pub number: u32,
    pub tracks: Vec<Track>,
}