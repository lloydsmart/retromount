use crate::core::content::DecodedContent;
use crate::core::source::SourceObject;
use crate::input::identify::InputIdentity;

pub trait InputDecoder: Send + Sync {
    fn supports(&self, identity: &InputIdentity) -> bool;

    fn decode(
        &self,
        object: &SourceObject,
        identity: &InputIdentity,
    ) -> Result<Vec<DecodedContent>, std::io::Error>;
}
