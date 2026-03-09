use thiserror::Error;

#[derive(Debug, Error)]
pub enum RetromountError {
    #[error("Failed to open config file: {0}")]
    ConfigFileError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ConfigParseError(#[from] serde_yaml::Error),

    #[error("Unsupported format")]
    UnsupportedFormat,

    #[error("Failed to detect input format")]
    DetectionFailed,

    #[error("Failed to load image: {0}")]
    LoadError(String),

    #[error("Plugin error: {0}")]
    PluginError(String),
}