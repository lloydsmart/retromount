use crate::core::source::SourceObject;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum InputIdentity {
    File,
    Directory,
    Archive,
    DiscImage,
    ChdDisc,
    IsoDisc,
    Text,
    Unknown,
}

pub trait InputIdentifier: Send + Sync {
    fn identify(&self, object: &SourceObject) -> Result<InputIdentity, std::io::Error>;
}
