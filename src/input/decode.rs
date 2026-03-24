use crate::core::content::Content;
use crate::core::source::SourceObject;
use crate::input::identify::InputIdentity;

pub trait InputDecoder: Send + Sync {
    fn supports(&self, identity: &InputIdentity) -> bool;

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<Content>, std::io::Error>;
}
