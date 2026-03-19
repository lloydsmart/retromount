use log::{debug, info};
use std::path::PathBuf;

use retromount::engine::inspect::run_phase3_inspect;
use retromount::engine::loader::Loader;
use retromount::engine::preview::run_phase3_preview;
use retromount::{RetromountError, ViewConfig};

fn main() -> Result<(), RetromountError> {
    env_logger::init();

    let args: Vec<_> = std::env::args_os().skip(1).collect();

    match args.as_slice() {
        [] => run_configured_views(),
        [command, path] if command.to_string_lossy() == "phase3-preview" => {
            let path = PathBuf::from(path);
            run_phase3_preview(&path)
        }
        [command, path] if command.to_string_lossy() == "inspect" => {
            let path = PathBuf::from(path);
            run_phase3_inspect(&path, false)
        }
        [command, path, flag]
            if command.to_string_lossy() == "inspect"
                && flag.to_string_lossy() == "--json" =>
        {
            let path = PathBuf::from(path);
            run_phase3_inspect(&path, true)
        }
        _ => Err(RetromountError::LoadError(
            "usage:\n  retromount\n  retromount phase3-preview <path>\n  retromount inspect <path> [--json]"
                .to_string(),
        )),
    }
}

fn run_configured_views() -> Result<(), RetromountError> {
    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;

    info!("Loaded {} view(s) from config", config.len());

    let loader = Loader::default();

    for view in &config {
        info!("View: {}", view.name);
        info!("  Source: {}", view.source.display());
        info!("  Mount: {}", view.mount.display());

        if view
            .source
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cue"))
        {
            let game = loader.load_game_image(&view.source, view.platform.clone())?;

            info!("  GameImage:");
            info!("    ID: {}", game.id);
            info!("    Title: {}", game.title);
            info!("    Platform: {:?}", game.platform);
            info!("    Discs: {}", game.discs.len());

            for disc in &game.discs {
                info!("    Disc {}: {} track(s)", disc.number, disc.tracks.len());

                for track in &disc.tracks {
                    info!(
                        "      Track {}: {:?}, sector size {}, size {}",
                        track.number, track.kind, track.sector_size, track.size
                    );
                }
            }
        } else {
            let files = loader.discover_path(&view.source)?;

            info!("  Discovered {} payload file(s)", files.len());

            for file in &files {
                info!(
                    "    - {} ({} bytes) [origin: {}]",
                    file.name,
                    file.size,
                    file.origin.display()
                );
            }
        }
    }

    Ok(())
}
