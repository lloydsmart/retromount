use crate::output::flat_presenter::FlatPresenter;
use crate::output::grouped_presenter::GroupedPresenter;
use crate::output::presenter_registry::PresenterRegistry;

pub fn default_presenter_registry() -> PresenterRegistry {
    let mut registry = PresenterRegistry::new();

    registry.register("grouped", || Box::new(GroupedPresenter::new()));
    registry.register("flat", || Box::new(FlatPresenter::new()));

    registry
}
