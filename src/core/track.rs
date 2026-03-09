#[derive(Debug, Clone)]
pub enum TrackType {
    Data,
    Audio,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub number: u32,
    pub kind: TrackType,
    pub size: u64,
}