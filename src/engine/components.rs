use crate::input::basic_decoder::BasicInputDecoder;
use crate::input::basic_identifier::BasicInputIdentifier;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentifier;
use crate::output::flat_presenter::FlatPresenter;
use crate::output::grouped_presenter::GroupedPresenter;
use crate::output::present::{OutputPresenter, PresenterKind};
use crate::policy::PolicySet;

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

pub fn default_pipeline_components() -> PipelineComponents {
    pipeline_components_for_presenter(PresenterKind::Grouped)
}

pub fn pipeline_components_for_presenter(kind: PresenterKind) -> PipelineComponents {
    let presenter: Box<dyn OutputPresenter> = match kind {
        PresenterKind::Grouped => Box::new(GroupedPresenter::new()),
        PresenterKind::Flat => Box::new(FlatPresenter::new()),
    };

    PipelineComponents {
        identifier: Box::new(BasicInputIdentifier::new()),
        decoder: Box::new(BasicInputDecoder::new()),
        presenter,
        policy: PolicySet::default(),
    }
}
