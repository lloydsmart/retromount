use crate::core::source::SourceObject;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InputSourceKind {
    Directory,
    Zip,
}

impl InputSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Directory => "Directory",
            Self::Zip => "Zip",
        }
    }
}

pub trait InputSource: Send + Sync {
    fn kind(&self) -> InputSourceKind;
    fn enumerate(&self) -> Result<Vec<SourceObject>, std::io::Error>;
}
