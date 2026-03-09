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
