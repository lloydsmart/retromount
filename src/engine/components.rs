use crate::core::content::Platform;
use crate::core::normalizer::NormalizationOptions;
use crate::engine::bootstrap::load_presentation;
use crate::input::basic_decoder::BasicInputDecoder;
use crate::input::basic_identifier::BasicInputIdentifier;
use crate::input::chd_disc_decoder::ChdDiscDecoder;
use crate::input::cue_disc_decoder::CueDiscDecoder;
use crate::input::decode::InputDecoder;
use crate::input::decoder_registry::DecoderRegistry;
use crate::input::identify::InputIdentifier;
use crate::input::iso_disc_decoder::IsoDiscDecoder;
use crate::output::presentation_spec::PresentationSpec;
use crate::policy::PolicySet;
use crate::RetromountError;

/// The concrete pipeline components used to run Retromount.
///
/// This provides a single composition boundary for built-in implementations,
/// allowing application entrypoints to use a selected declarative presentation
/// without constructing target-specific layout implementations.
pub struct PipelineComponents {
    pub identifier: Box<dyn InputIdentifier>,
    pub decoder: Box<dyn InputDecoder>,
    pub presentation: PresentationSpec,
    pub normalization: NormalizationOptions,
    pub policy: PolicySet,
}

pub fn default_pipeline_components() -> Result<PipelineComponents, RetromountError> {
    pipeline_components("grouped")
}

pub fn pipeline_components(presentation_name: &str) -> Result<PipelineComponents, RetromountError> {
    pipeline_components_with_media(presentation_name, None)
}

pub fn pipeline_components_with_media(
    presentation_name: &str,
    media_hint: Option<crate::core::content::DiscMedia>,
) -> Result<PipelineComponents, RetromountError> {
    let document = load_presentation(presentation_name).map_err(RetromountError::LoadError)?;

    let mut decoder = DecoderRegistry::new();
    decoder.register(ChdDiscDecoder::new());
    decoder.register(if presentation_name == "opl" {
        CueDiscDecoder::for_opl()
    } else {
        CueDiscDecoder::new()
    });
    if presentation_name == "opl" {
        decoder.register(IsoDiscDecoder::new(
            media_hint.unwrap_or(crate::core::content::DiscMedia::Dvd),
        ));
    }
    decoder.register(BasicInputDecoder::new());

    let normalization = match presentation_name {
        "opl" => NormalizationOptions {
            platform_hint: Some(Platform::Ps2),
        },
        "duckstation" => NormalizationOptions {
            platform_hint: Some(Platform::Ps1),
        },
        _ => NormalizationOptions::default(),
    };

    Ok(PipelineComponents {
        identifier: Box::new(BasicInputIdentifier::new()),
        decoder: Box::new(decoder),
        presentation: document.spec,
        normalization,
        policy: PolicySet::default(),
    })
}

pub fn pipeline_components_for_presentation(
    presentation_name: &str,
) -> Result<PipelineComponents, RetromountError> {
    pipeline_components(presentation_name)
}
