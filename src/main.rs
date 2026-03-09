use log::{debug, info};
use std::path::PathBuf;

use retromount::core::reader::Reader;
use retromount::readers::dir_reader::DirReader;
use retromount::{RetromountError, ViewConfig};

fn main() -> Result<(), RetromountError> {
    env_logger::init();

    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;

    info!("Loaded config: {:?}", config);

    // --- temporary reader test ---
    let path = PathBuf::from("Cargo.toml");
    info!("Testing DirReader with {:?}", path);

    let mut reader = DirReader::open(&path)?;

    let mut buf = vec![0u8; 64];
    let bytes = reader.read_at(0, &mut buf)?;

    info!("Read {} bytes", bytes);
    debug!("Data: {}", String::from_utf8_lossy(&buf));

    Ok(())
}
