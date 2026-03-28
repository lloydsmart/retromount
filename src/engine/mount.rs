use std::path::Path;

use log::info;

use crate::engine::components::default_pipeline_components;
use crate::engine::pipeline::run_pipeline;
use crate::engine::preview::build_input_source;
use crate::error::RetromountError;
use crate::mount::session::MountSession;

#[cfg(target_os = "linux")]
use crate::mount::adapter::FilesystemAdapter;
#[cfg(target_os = "linux")]
use crate::mount::fuse_fs::RetromountFuseFs;

pub fn run_mount_command(input: &Path, mountpoint: &Path) -> Result<(), RetromountError> {
    let source = build_input_source(input)?;
    let components = default_pipeline_components();

    let root = run_pipeline(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        components.presenter.as_ref(),
    )?;

    let session = MountSession::from_root(&root);
    let root_children = session
        .children(session.root_inode())
        .map(|children| children.len())
        .unwrap_or(0);

    info!("Prepared mount input: {}", input.display());
    info!("Prepared mountpoint: {}", mountpoint.display());
    info!("Indexed VFS nodes: {}", session.node_count());
    info!("Root inode: {}", session.root_inode());
    info!("Root child entries: {}", root_children);

    mount_session(session, mountpoint)
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
