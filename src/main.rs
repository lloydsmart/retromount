use serde::Deserialize;
use log::info;

#[derive(Debug, Deserialize)]
struct View {
    name: String,
    source: String,
    mount: String,
    translator: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let file = std::fs::File::open("retromount.yaml")?;
    let config: Vec<View> = serde_yaml::from_reader(file)?;
    info!("Loaded config: {:?}", config);
    Ok(())
}
