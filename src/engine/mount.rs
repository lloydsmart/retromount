use std::path::Path;

use log::info;

use crate::engine::components::pipeline_components_for_presentation;
use crate::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use crate::engine::preview::build_input_source;
use crate::error::RetromountError;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentifier;
use crate::input::source::InputSource;
use crate::mount::session::MountSession;
use crate::output::plugin_registry::PluginRegistry;
use crate::policy::PolicySet;

#[cfg(target_os = "linux")]
use crate::mount::adapter::FilesystemAdapter;
#[cfg(target_os = "linux")]
use crate::mount::fuse_fs::RetromountFuseFs;

pub fn run_mount_command(
    input: &Path,
    mountpoint: &Path,
    presentation_name: &str,
) -> Result<(), RetromountError> {
    run_mount_command_with_plugins(input, mountpoint, presentation_name, None)
}

pub fn run_mount_command_with_plugins(
    input: &Path,
    mountpoint: &Path,
    presentation_name: &str,
    plugin_registry: Option<&PluginRegistry>,
) -> Result<(), RetromountError> {
    let session = prepare_mount_session_with_plugins(input, presentation_name, plugin_registry)?;
    let root_children = session
        .children(session.root_inode())
        .map(|children| children.len())
        .unwrap_or(0);

    info!("Prepared mount input: {}", input.display());
    info!("Prepared mountpoint: {}", mountpoint.display());
    info!("Presentation view: {}", presentation_name);
    info!("Indexed VFS nodes: {}", session.node_count());
    info!("Root inode: {}", session.root_inode());
    info!("Root child entries: {}", root_children);

    mount_session(session, mountpoint)
}

pub fn prepare_mount_session(
    input: &Path,
    presentation_name: &str,
) -> Result<MountSession, RetromountError> {
    prepare_mount_session_with_plugins(input, presentation_name, None)
}

pub fn prepare_mount_session_with_plugins(
    input: &Path,
    presentation_name: &str,
    plugin_registry: Option<&PluginRegistry>,
) -> Result<MountSession, RetromountError> {
    let source = build_input_source(input)?;
    let components = pipeline_components_for_presentation(presentation_name)?;

    prepare_mount_session_from_presentation_with_normalization(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &components.presentation,
        &components.policy,
        plugin_registry,
        &components.normalization,
    )
}

pub fn prepare_mount_session_from_presentation(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presentation: &crate::output::presentation_spec::PresentationSpec,
    policy: &PolicySet,
    plugin_registry: Option<&PluginRegistry>,
) -> Result<MountSession, RetromountError> {
    prepare_mount_session_from_presentation_with_normalization(
        source,
        identifier,
        decoder,
        presentation,
        policy,
        plugin_registry,
        &Default::default(),
    )
}

fn prepare_mount_session_from_presentation_with_normalization(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presentation: &crate::output::presentation_spec::PresentationSpec,
    policy: &PolicySet,
    plugin_registry: Option<&PluginRegistry>,
    normalization: &crate::core::normalizer::NormalizationOptions,
) -> Result<MountSession, RetromountError> {
    let root = run_pipeline_with_presentation_options(
        source,
        identifier,
        decoder,
        presentation,
        policy,
        &PipelineOptions {
            normalization: normalization.clone(),
            plugin_registry,
        },
    )
    .map_err(|err| RetromountError::LoadError(err.to_string()))?
    .output_vfs;

    Ok(MountSession::from_root(&root))
}

#[cfg(target_os = "linux")]
fn mount_session(session: MountSession, mountpoint: &Path) -> Result<(), RetromountError> {
    RetromountFuseFs::new(session).mount(mountpoint)
}

#[cfg(not(target_os = "linux"))]
fn mount_session(_session: MountSession, mountpoint: &Path) -> Result<(), RetromountError> {
    Err(RetromountError::LoadError(format!(
        "mount is only supported on Linux (FUSE); cannot mount to {} on this platform. Try `retromount phase3-preview <path>` instead.",
        mountpoint.display()
    )))
}
