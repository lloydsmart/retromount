use crate::engine::bootstrap::default_presenter_registry;
use crate::input::basic_decoder::BasicInputDecoder;
use crate::input::basic_identifier::BasicInputIdentifier;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentifier;
use crate::output::present::OutputPresenter;
use crate::policy::PolicySet;
use crate::RetromountError;

/// The concrete pipeline components used to run Retromount.
///
/// This provides a single composition boundary for built-in implementations,
/// allowing application entrypoints to depend on traits rather than directly
/// constructing concrete decoders, identifiers, and presenters.
pub struct PipelineComponents {
    pub identifier: Box<dyn InputIdentifier>,
    pub decoder: Box<dyn InputDecoder>,
    pub presenter: Box<dyn OutputPresenter>,
    pub policy: PolicySet,
}

pub fn default_pipeline_components() -> Result<PipelineComponents, RetromountError> {
    pipeline_components_for_presenter("grouped")
}

pub fn pipeline_components_for_presenter(
    name: &str,
) -> Result<PipelineComponents, RetromountError> {
    let registry = default_presenter_registry();

    let presenter = registry.get(name).ok_or_else(|| {
        RetromountError::LoadError(format!(
            "unsupported view '{name}'; expected one of: {}",
            registry.names().join(", ")
        ))
    })?;

    Ok(PipelineComponents {
        identifier: Box::new(BasicInputIdentifier::new()),
        decoder: Box::new(BasicInputDecoder::new()),
        presenter,
        policy: PolicySet::default(),
    })
}
