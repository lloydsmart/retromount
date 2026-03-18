use crate::core::source::SourceObject;

pub trait InputSource: Send + Sync {
    fn enumerate(&self) -> Result<Vec<SourceObject>, std::io::Error>;
}
