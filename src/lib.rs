use serde::Deserialize;
use std::path::PathBuf;

use crate::core::platform::Platform;

pub mod builtin_inputs;
pub mod core;
pub mod engine;
pub mod error;
pub mod input;
pub mod output;
pub mod readers;

pub use error::RetromountError;

#[derive(Debug, Deserialize)]
pub struct ViewConfig {
    pub name: String,
    pub source: PathBuf,
    pub mount: PathBuf,
    pub platform: Platform,
}
