use log::{debug, info};
use std::path::PathBuf;

use retromount::engine::loader::Loader;
use retromount::{RetromountError, ViewConfig};

fn main() -> Result<(), RetromountError> {
    env_logger::init();

    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;

    info!("Loaded {} view(s) from config", config.len());

    let loader = Loader::default();

    for view in &config {
        info!("View: {}", view.name);
        info!("  Source: {}", view.source.display());
        info!("  Mount: {}", view.mount.display());

        let files = loader.discover_path(&view.source)?;

        info!("  Discovered {} payload file(s)", files.len());

        for file in &files {
            info!(
                "    - {} ({} bytes) [origin: {}]",
                file.name,
                file.size,
                file.origin.display()
            );
        }
    }

    Ok(())
}
