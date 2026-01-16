use log::{info, debug};
use serde::Deserialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct ViewConfig {
    pub name: String,
    pub source: PathBuf,
    pub mount: PathBuf,
    pub translator: String,
}

#[derive(Error, Debug)]
pub enum RetroMountError {
    #[error("Failed to open config file: {0}")]
    ConfigFileError(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    ConfigParseError(#[from] serde_yaml::Error),
}

fn main() -> Result<(), RetroMountError> {
    env_logger::init();
    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;
    info!("Loaded config: {:?}", config);
    Ok(())
}
