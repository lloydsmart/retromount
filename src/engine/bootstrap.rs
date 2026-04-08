use crate::output::basic_encoder::BasicEncoder;
use crate::output::encoder_registry::EncoderRegistry;
use crate::output::flat_presenter::FlatPresenter;
use crate::output::grouped_presenter::GroupedPresenter;
use crate::output::presenter_registry::PresenterRegistry;

pub fn default_encoder_registry() -> EncoderRegistry {
    let mut registry = EncoderRegistry::new();

    registry.register("basic", || Box::new(BasicEncoder::new()));

    registry
}

pub fn default_presenter_registry() -> PresenterRegistry {
    let mut registry = PresenterRegistry::new();

    registry.register("grouped", || {
        let encoder = default_encoder_registry()
            .get("basic")
            .expect("encoder 'basic' is not registered");

        Box::new(GroupedPresenter::new(encoder))
    });

    registry.register("flat", || {
        let encoder = default_encoder_registry()
            .get("basic")
            .expect("encoder 'basic' is not registered");

        Box::new(FlatPresenter::new(encoder))
    });

    registry
}
