use std::path::Path;

use log::info;

use crate::engine::components::pipeline_components_for_presenter;
use crate::engine::pipeline::{run_pipeline, run_pipeline_with_options, PipelineOptions};
use crate::engine::preview::build_input_source;
use crate::error::RetromountError;
use crate::input::decode::InputDecoder;
use crate::input::identify::InputIdentifier;
use crate::input::source::InputSource;
use crate::mount::session::MountSession;
use crate::output::plugin_registry::PluginRegistry;
use crate::output::present::OutputPresenter;
use crate::policy::PolicySet;

#[cfg(target_os = "linux")]
use crate::mount::adapter::FilesystemAdapter;
#[cfg(target_os = "linux")]
use crate::mount::fuse_fs::RetromountFuseFs;

pub fn run_mount_command(
    input: &Path,
    mountpoint: &Path,
    presenter_name: &str,
) -> Result<(), RetromountError> {
    run_mount_command_with_plugins(input, mountpoint, presenter_name, None)
}

pub fn run_mount_command_with_plugins(
    input: &Path,
    mountpoint: &Path,
    presenter_name: &str,
    plugin_registry: Option<&PluginRegistry>,
) -> Result<(), RetromountError> {
    let session = prepare_mount_session_with_plugins(input, presenter_name, plugin_registry)?;
    let root_children = session
        .children(session.root_inode())
        .map(|children| children.len())
        .unwrap_or(0);

    info!("Prepared mount input: {}", input.display());
    info!("Prepared mountpoint: {}", mountpoint.display());
    info!("Presenter view: {}", presenter_name);
    info!("Indexed VFS nodes: {}", session.node_count());
    info!("Root inode: {}", session.root_inode());
    info!("Root child entries: {}", root_children);

    mount_session(session, mountpoint)
}

pub fn prepare_mount_session(
    input: &Path,
    presenter_name: &str,
) -> Result<MountSession, RetromountError> {
    prepare_mount_session_with_plugins(input, presenter_name, None)
}

pub fn prepare_mount_session_with_plugins(
    input: &Path,
    presenter_name: &str,
    plugin_registry: Option<&PluginRegistry>,
) -> Result<MountSession, RetromountError> {
    let source = build_input_source(input)?;
    let components = pipeline_components_for_presenter(presenter_name)?;

    prepare_mount_session_from_pipeline(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        components.presenter.as_ref(),
        &components.policy,
        plugin_registry,
    )
}

pub fn prepare_mount_session_from_pipeline(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
    plugin_registry: Option<&PluginRegistry>,
) -> Result<MountSession, RetromountError> {
    let root = match plugin_registry {
        Some(plugin_registry) => {
            run_pipeline_with_options(
                source,
                identifier,
                decoder,
                presenter,
                policy,
                &PipelineOptions {
                    normalization: Default::default(),
                    plugin_registry: Some(plugin_registry),
                },
            )?
            .output_vfs
        }
        None => run_pipeline(source, identifier, decoder, presenter, policy)?,
    };

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
