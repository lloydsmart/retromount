use serde::Deserialize;
use std::path::PathBuf;

pub mod core;
pub mod engine;
pub mod error;
pub mod inputs;
pub mod readers;

pub use error::RetromountError;

#[derive(Debug, Deserialize)]
pub struct ViewConfig {
    pub name: String,
    pub source: PathBuf,
    pub mount: PathBuf,
}
