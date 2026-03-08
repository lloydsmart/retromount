use log::{debug, info};
use std::path::PathBuf;

use retromount::{RetroMountError, ViewConfig};

fn main() -> Result<(), RetroMountError> {
    env_logger::init();

    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;

    info!("Loaded config: {:?}", config);

    Ok(())
}