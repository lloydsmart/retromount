use log::{debug, info};
use std::path::PathBuf;

use retromount::core::content::{
    ContentMeta, GamePart, NormalizedContent, Platform as ContentPlatform,
};
use retromount::core::normalizer::NormalizationOptions;
use retromount::core::platform::Platform as ConfigPlatform;
use retromount::engine::components::default_pipeline_components;
use retromount::engine::inspect::run_phase3_inspect;
use retromount::engine::mount::run_mount_command;
use retromount::engine::pipeline::run_pipeline_with_options;
use retromount::engine::preview::{build_input_source, run_phase3_preview, write_vfs_tree};
use retromount::output::present::PresenterKind;
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
            run_phase3_inspect(&path, false, PresenterKind::Grouped)
        }
        [command, path, flag]
            if command.to_string_lossy() == "inspect" && flag.to_string_lossy() == "--json" =>
        {
            let path = PathBuf::from(path);
            run_phase3_inspect(&path, true, PresenterKind::Grouped)
        }
        [command, path, view_flag, view]
            if command.to_string_lossy() == "inspect" && view_flag.to_string_lossy() == "--view" =>
        {
            let path = PathBuf::from(path);
            let presenter_kind = parse_presenter_kind(view)?;
            run_phase3_inspect(&path, false, presenter_kind)
        }
        [command, path, flag, view_flag, view]
            if command.to_string_lossy() == "inspect"
                && flag.to_string_lossy() == "--json"
                && view_flag.to_string_lossy() == "--view" =>
        {
            let path = PathBuf::from(path);
            let presenter_kind = parse_presenter_kind(view)?;
            run_phase3_inspect(&path, true, presenter_kind)
        }
        [command, input, mountpoint] if command.to_string_lossy() == "mount" => {
            let input = PathBuf::from(input);
            let mountpoint = PathBuf::from(mountpoint);
            run_mount_command(&input, &mountpoint, PresenterKind::Grouped)
        }
        [command, input, mountpoint, view_flag, view]
            if command.to_string_lossy() == "mount" && view_flag.to_string_lossy() == "--view" =>
        {
            let input = PathBuf::from(input);
            let mountpoint = PathBuf::from(mountpoint);
            let presenter_kind = parse_presenter_kind(view)?;
            run_mount_command(&input, &mountpoint, presenter_kind)
        }
        _ => Err(RetromountError::LoadError(
            "usage:\n  retromount\n  retromount phase3-preview <path>\n  retromount inspect <path> [--json] [--view <grouped|flat>]\n  retromount mount <input> <mountpoint> [--view <grouped|flat>]"
                .to_string(),
        )),
    }
}

fn parse_presenter_kind(value: &std::ffi::OsStr) -> Result<PresenterKind, RetromountError> {
    let value = value.to_string_lossy();
    PresenterKind::parse(&value).ok_or_else(|| {
        RetromountError::LoadError(format!(
            "unsupported view '{value}'; expected one of: grouped, flat"
        ))
    })
}

fn run_configured_views() -> Result<(), RetromountError> {
    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;

    info!("Loaded {} view(s) from config", config.len());

    let components = default_pipeline_components();

    for view in &config {
        info!("View: {}", view.name);
        info!("  Source: {}", view.source.display());
        info!("  Mount: {}", view.mount.display());

        let source = build_input_source(&view.source)?;

        let trace = run_pipeline_with_options(
            source.as_ref(),
            components.identifier.as_ref(),
            components.decoder.as_ref(),
            components.presenter.as_ref(),
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
        write_vfs_tree(&mut rendered, &trace.output_vfs)
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

fn log_content_summary(content: &NormalizedContent) {
    match content {
        NormalizedContent::Game(game) => {
            info!("  Game:");
            info!("    ID: {}", game.id);
            info!("    Title: {}", game.title);
            info!("    Platform: {}", game.platform);
            info!("    Parts: {}", game.parts.len());

            for part in &game.parts {
                match part {
                    GamePart::Rom(rom) => {
                        info!("    Rom: {} ({} bytes)", rom.source.file_name(), rom.size);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_presenter_kind() {
        assert_eq!(
            parse_presenter_kind(std::ffi::OsStr::new("grouped")).unwrap(),
            PresenterKind::Grouped
        );
        assert_eq!(
            parse_presenter_kind(std::ffi::OsStr::new("flat")).unwrap(),
            PresenterKind::Flat
        );
        assert!(parse_presenter_kind(std::ffi::OsStr::new("weird")).is_err());
    }
}
