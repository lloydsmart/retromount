use crate::input::basic_decoder::BasicInputDecoder;
use crate::input::basic_identifier::BasicInputIdentifier;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentifier;
use crate::output::basic_encoder::BasicEncoder;
use crate::output::generic_presenter::GenericPresenter;
use crate::output::present::OutputPresenter;

/// The concrete pipeline components used to run Retromount.
///
/// This provides a single composition boundary for built-in implementations,
/// allowing application entrypoints to depend on traits rather than directly
/// constructing concrete decoders, identifiers, presenters, or encoders.
pub struct PipelineComponents {
    pub identifier: Box<dyn InputIdentifier>,
    pub decoder: Box<dyn InputDecoder>,
    pub presenter: Box<dyn OutputPresenter>,
}

/// Builds the default built-in pipeline component stack.
///
/// This is the current compile-time composition point for Retromount's
/// standard identifier, decoder, presenter, and encoder implementations.
/// Future plugin-style composition can replace or extend this boundary
/// without requiring changes in application entrypoints.
pub fn default_pipeline_components() -> PipelineComponents {
    PipelineComponents {
        identifier: Box::new(BasicInputIdentifier::new()),
        decoder: Box::new(BasicInputDecoder::new()),
        presenter: Box::new(GenericPresenter::new(BasicEncoder::new())),
    }
}
