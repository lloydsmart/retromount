use std::path::Path;

use crate::error::RetromountError;
use crate::output::plugin_discovery::{build_registry_from_discovery, discover_encoder_plugins};
use crate::output::plugin_registry::PluginRegistry;

pub fn load_plugin_registry(path: &Path) -> Result<PluginRegistry, RetromountError> {
    let report = discover_encoder_plugins(path);

    if !report.rejected.is_empty() {
        let details = report
            .rejected
            .iter()
            .map(|rejected| format!("{}: {:?}", rejected.executable.display(), rejected.error))
            .collect::<Vec<_>>()
            .join(", ");

        return Err(RetromountError::PluginError(format!(
            "failed to load plugin(s) from '{}': {}",
            path.display(),
            details
        )));
    }

    build_registry_from_discovery(report)
        .map_err(|error| RetromountError::PluginError(format!("{error:?}")))
}
