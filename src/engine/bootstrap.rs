use crate::output::flat_presenter::FlatPresenter;
use crate::output::grouped_presenter::GroupedPresenter;
use crate::output::present::OutputPresenter;
use crate::output::presenter_registry::PresenterRegistry;

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

pub fn build_presenter(presenter_name: &str) -> Result<Box<dyn OutputPresenter>, String> {
    match presenter_name {
        "grouped" => Ok(Box::new(GroupedPresenter::new()) as Box<dyn OutputPresenter>),
        "flat" => Ok(Box::new(FlatPresenter::new()) as Box<dyn OutputPresenter>),
        _ => {
            let presenter_registry = default_presenter_registry();
            Err(format!(
                "unsupported view '{presenter_name}'; expected one of: {}",
                presenter_registry.names().join(", ")
            ))
        }
    }
}
