use serde::Serialize;
use crate::core::source::SourceObject;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum InputIdentity {
    File,
    Directory,
    Archive,
    DiscImage,
    Text,
    Unknown,
}

pub trait InputIdentifier: Send + Sync {
    fn identify(&self, object: &SourceObject) -> Result<InputIdentity, std::io::Error>;
}
