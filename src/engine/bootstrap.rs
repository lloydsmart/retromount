use crate::output::basic_encoder::BasicEncoder;
use crate::output::encoder_registry::EncoderRegistry;
use crate::output::flat_presenter::FlatPresenter;
use crate::output::grouped_presenter::GroupedPresenter;
use crate::output::present::OutputPresenter;
use crate::output::presenter_registry::PresenterRegistry;

pub fn default_encoder_registry() -> EncoderRegistry {
    let mut registry = EncoderRegistry::new();

    registry.register("basic", || Box::new(BasicEncoder::new()));

    registry
}

pub fn default_presenter_registry() -> PresenterRegistry {
    let mut registry = PresenterRegistry::new();

    registry.register("grouped", || {
        Box::new(GroupedPresenter) as Box<dyn OutputPresenter>
    });

    registry.register("flat", || {
        Box::new(FlatPresenter) as Box<dyn OutputPresenter>
    });

    registry
}

pub fn build_presenter(
    presenter_name: &str,
    encoder_name: &str,
) -> Result<Box<dyn OutputPresenter>, String> {
    let encoder_registry = default_encoder_registry();

    let encoder = encoder_registry.get(encoder_name).ok_or_else(|| {
        format!(
            "unsupported encoder '{encoder_name}'; expected one of: {}",
            encoder_registry.names().join(", ")
        )
    })?;

    match presenter_name {
        "grouped" => Ok(Box::new(GroupedPresenter::new(encoder))),
        "flat" => Ok(Box::new(FlatPresenter::new(encoder))),
        _ => {
            let presenter_registry = default_presenter_registry();
            Err(format!(
                "unsupported view '{presenter_name}'; expected one of: {}",
                presenter_registry.names().join(", ")
            ))
        }
    }
}
