use thiserror::Error;

#[derive(Debug, Error)]
pub enum RetromountError {
    #[error("Unsupported format")]
    UnsupportedFormat,

    #[error("Failed to detect input format")]
    DetectionFailed,

    #[error("Failed to load image: {0}")]
    LoadError(String),

    #[error("Plugin error: {0}")]
    PluginError(String),
}