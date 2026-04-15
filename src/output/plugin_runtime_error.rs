use std::path::PathBuf;

use thiserror::Error;

use crate::output::plugin_protocol::ProtocolError;

#[derive(Debug, Error)]
pub enum PluginRuntimeError {
    #[error("plugin executable not found: {path}")]
    ExecutableNotFound { path: PathBuf },

    #[error("failed to spawn plugin executable '{path}': {message}")]
    SpawnFailed { path: PathBuf, message: String },

    #[error("failed to communicate with plugin executable '{path}': {message}")]
    Io { path: PathBuf, message: String },

    #[error("plugin executable '{path}' exited with non-zero status {status}: {stderr}")]
    NonZeroExit {
        path: PathBuf,
        status: i32,
        stderr: String,
    },

    #[error("plugin executable '{path}' produced invalid JSON: {message}")]
    InvalidJson { path: PathBuf, message: String },

    #[error("plugin executable '{path}' returned an unexpected response: {message}")]
    UnexpectedResponse { path: PathBuf, message: String },

    #[error("plugin protocol error")]
    Protocol { error: ProtocolError },
}

impl From<ProtocolError> for PluginRuntimeError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol { error }
    }
}

impl PluginRuntimeError {
    pub fn to_protocol_error(&self) -> ProtocolError {
        match self {
            Self::Protocol { error } => error.clone(),
            other => ProtocolError::InternalPluginError {
                message: other.to_string(),
            },
        }
    }
}
