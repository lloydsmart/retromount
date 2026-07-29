use log::{debug, info};
use std::path::PathBuf;

use retromount::core::content::{
    ContentMeta, GamePart, NormalizedContent, Platform as ContentPlatform,
};
use retromount::core::normalizer::NormalizationOptions;
use retromount::core::platform::Platform as ConfigPlatform;
use retromount::engine::bootstrap::{build_presentation_spec, built_in_presentation_names};
use retromount::engine::components::pipeline_components;
use retromount::engine::inspect::{run_phase3_inspect, run_phase3_inspect_with_plugins};
use retromount::engine::mount::{run_mount_command, run_mount_command_with_plugins};
use retromount::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use retromount::engine::plugins::load_plugin_registry;
use retromount::engine::preview::{
    build_input_source, run_phase3_preview, run_phase3_preview_with_plugins, write_vfs_tree,
};
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
        [command, path, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "phase3-preview"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let path = PathBuf::from(path);
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_phase3_preview_with_plugins(&path, Some(&plugin_registry))
        }

        [command, path] if command.to_string_lossy() == "inspect" => {
            let path = PathBuf::from(path);
            run_phase3_inspect(&path, false, "grouped")
        }
        [command, path, flag]
            if command.to_string_lossy() == "inspect" && flag.to_string_lossy() == "--json" =>
        {
            let path = PathBuf::from(path);
            run_phase3_inspect(&path, true, "grouped")
        }
        [command, path, view_flag, view]
            if command.to_string_lossy() == "inspect" && view_flag.to_string_lossy() == "--view" =>
        {
            let path = PathBuf::from(path);
            let presenter_name = parse_presenter_name(view)?;
            run_phase3_inspect(&path, false, &presenter_name)
        }
        [command, path, flag, view_flag, view]
            if command.to_string_lossy() == "inspect"
                && flag.to_string_lossy() == "--json"
                && view_flag.to_string_lossy() == "--view" =>
        {
            let path = PathBuf::from(path);
            let presenter_name = parse_presenter_name(view)?;
            run_phase3_inspect(&path, true, &presenter_name)
        }
        [command, path, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "inspect"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let path = PathBuf::from(path);
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_phase3_inspect_with_plugins(&path, false, "grouped", Some(&plugin_registry))
        }
        [command, path, json_flag, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "inspect"
                && json_flag.to_string_lossy() == "--json"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let path = PathBuf::from(path);
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_phase3_inspect_with_plugins(&path, true, "grouped", Some(&plugin_registry))
        }
        [command, path, view_flag, view, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "inspect"
                && view_flag.to_string_lossy() == "--view"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let path = PathBuf::from(path);
            let presenter_name = parse_presenter_name(view)?;
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_phase3_inspect_with_plugins(&path, false, &presenter_name, Some(&plugin_registry))
        }
        [command, path, json_flag, view_flag, view, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "inspect"
                && json_flag.to_string_lossy() == "--json"
                && view_flag.to_string_lossy() == "--view"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let path = PathBuf::from(path);
            let presenter_name = parse_presenter_name(view)?;
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_phase3_inspect_with_plugins(&path, true, &presenter_name, Some(&plugin_registry))
        }

        [command, input, mountpoint] if command.to_string_lossy() == "mount" => {
            let input = PathBuf::from(input);
            let mountpoint = PathBuf::from(mountpoint);
            run_mount_command(&input, &mountpoint, "grouped")
        }
        [command, input, mountpoint, view_flag, view]
            if command.to_string_lossy() == "mount" && view_flag.to_string_lossy() == "--view" =>
        {
            let input = PathBuf::from(input);
            let mountpoint = PathBuf::from(mountpoint);
            let presenter_name = parse_presenter_name(view)?;
            run_mount_command(&input, &mountpoint, &presenter_name)
        }
        [command, input, mountpoint, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "mount"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let input = PathBuf::from(input);
            let mountpoint = PathBuf::from(mountpoint);
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_mount_command_with_plugins(&input, &mountpoint, "grouped", Some(&plugin_registry))
        }
        [command, input, mountpoint, view_flag, view, plugin_flag, plugin_dir]
            if command.to_string_lossy() == "mount"
                && view_flag.to_string_lossy() == "--view"
                && plugin_flag.to_string_lossy() == "--plugin-dir" =>
        {
            let input = PathBuf::from(input);
            let mountpoint = PathBuf::from(mountpoint);
            let presenter_name = parse_presenter_name(view)?;
            let plugin_dir = PathBuf::from(plugin_dir);
            let plugin_registry = load_plugin_registry(&plugin_dir)?;
            run_mount_command_with_plugins(
                &input,
                &mountpoint,
                &presenter_name,
                Some(&plugin_registry),
            )
        }

        _ => Err(RetromountError::LoadError(
            "usage:\n  retromount\n  retromount phase3-preview <path> [--plugin-dir <dir>]\n  retromount inspect <path> [--json] [--view <grouped|flat>] [--plugin-dir <dir>]\n  retromount mount <input> <mountpoint> [--view <grouped|flat>] [--plugin-dir <dir>]"
                .to_string(),
        )),
    }
}

fn parse_presenter_name(value: &std::ffi::OsStr) -> Result<String, RetromountError> {
    let value = value.to_string_lossy().to_string();

    if build_presentation_spec(&value).is_ok() {
        Ok(value)
    } else {
        Err(RetromountError::LoadError(format!(
            "unsupported view '{value}'; expected one of: {}",
            built_in_presentation_names().join(", ")
        )))
    }
}

fn run_configured_views() -> Result<(), RetromountError> {
    let config_path = PathBuf::from("retromount.yaml");
    debug!("Loading config from: {:?}", config_path);

    let file = std::fs::File::open(&config_path)?;
    let config: Vec<ViewConfig> = serde_yaml::from_reader(file)?;

    info!("Loaded {} view(s) from config", config.len());

    for view in &config {
        let presenter_name = view.presenter.as_deref().unwrap_or("grouped");

        info!("View: {}", view.name);
        info!("  Source: {}", view.source.display());
        info!("  Mount: {}", view.mount.display());
        info!("  Presenter: {}", presenter_name);

        let components = pipeline_components(presenter_name)?;
        let source = build_input_source(&view.source)?;

        let trace = run_pipeline_with_presentation_options(
            source.as_ref(),
            components.identifier.as_ref(),
            components.decoder.as_ref(),
            &components.presentation,
            &components.policy,
            &pipeline_options_for_view(view),
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

fn pipeline_options_for_view(view: &ViewConfig) -> PipelineOptions<'static> {
    PipelineOptions {
        normalization: NormalizationOptions {
            platform_hint: Some(map_view_platform(&view.platform)),
        },
        plugin_registry: None,
    }
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
