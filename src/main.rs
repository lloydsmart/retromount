use log::{debug, info};
use std::path::PathBuf;

use crate::core::source::SourceRef;

use retromount::core::content::{Content, GamePart, Platform as ContentPlatform};
use retromount::core::normalizer::NormalizationOptions;
use retromount::core::platform::Platform as ConfigPlatform;
use retromount::engine::inspect::run_phase3_inspect;
use retromount::engine::pipeline::run_pipeline_with_options;
use retromount::engine::preview::{build_input_source, run_phase3_preview, write_vfs_tree};
use retromount::input::basic_decoder::BasicInputDecoder;
use retromount::input::basic_identifier::BasicInputIdentifier;
use retromount::output::basic_encoder::BasicEncoder;
use retromount::output::generic_presenter::GenericPresenter;
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
            if command.to_string_lossy() == "inspect" && flag.to_string_lossy() == "--json" =>
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

    let identifier = BasicInputIdentifier::new();
    let decoder = BasicInputDecoder::new();
    let presenter = GenericPresenter::new(BasicEncoder::new());

    for view in &config {
        info!("View: {}", view.name);
        info!("  Source: {}", view.source.display());
        info!("  Mount: {}", view.mount.display());

        let source = build_input_source(&view.source)?;

        let trace = run_pipeline_with_options(
            source.as_ref(),
            &identifier,
            &decoder,
            &presenter,
            &NormalizationOptions {
                platform_hint: Some(map_view_platform(&view.platform)),
            },
        )
        .map_err(RetromountError::ConfigFileError)?;

        info!("  Objects: {}", trace.objects.len());
        info!("  Normalized content: {}", trace.normalized.len());

        for content in &trace.normalized {
            log_content_summary(content);
        }

        let mut rendered = Vec::new();
        write_vfs_tree(&mut rendered, &trace.presented)
            .map_err(RetromountError::ConfigFileError)?;
        let rendered = String::from_utf8(rendered)
            .map_err(|err| RetromountError::LoadError(err.to_string()))?;

        for line in rendered.lines() {
            info!("  VFS {}", line);
        }
    }

    Ok(())
}

fn map_view_platform(platform: &ConfigPlatform) -> ContentPlatform {
    match platform {
        ConfigPlatform::PlayStation => ContentPlatform::Ps1,
        ConfigPlatform::SuperNintendo => ContentPlatform::Snes,
        ConfigPlatform::MegaDrive => ContentPlatform::Megadrive,
        _ => ContentPlatform::Unknown,
    }
}

fn log_content_summary(content: &Content) {
    match content {
        Content::Game(game) => {
            info!("  Game:");
            info!("    ID: {}", game.id);
            info!("    Title: {}", game.title);
            info!("    Platform: {}", game.platform);
            info!("    Parts: {}", game.parts.len());

            for part in &game.parts {
                match part {
                    GamePart::Rom(rom) => {
                        info!(
                            "    Rom: {} ({} bytes)",
                            file_name_from_source(&rom.source),
                            rom.size
                        );
                    }
                    GamePart::Disc(disc) => {
                        info!("    Disc {}: {}", disc.disc_number, disc.source);
                    }
                }
            }
        }
        other => {
            info!("  Content: {:?}", other.kind());
            info!("    ID: {}", other.id());
            info!("    Source: {}", other.source());
        }
    }
}

fn file_name_from_source(source: &SourceRef) -> String {
    let normalized = source.0.replace('\\', "/");

    let leaf = match normalized.rsplit_once('#') {
        Some((_, member)) => member,
        None => normalized.rsplit('/').next().unwrap_or(&normalized),
    };

    leaf.to_string()
}
