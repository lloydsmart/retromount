use crate::core::content::Content;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFile {
    pub name: String,
    pub size: u64,
}

pub trait OutputEncoder: Send + Sync {
    fn can_encode(&self, content: &Content) -> bool;
    fn encode(&self, content: &Content) -> Result<EncodedFile, std::io::Error>;
}
