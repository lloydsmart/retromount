use std::path::Path;

use log::info;

use crate::engine::components::default_pipeline_components;
use crate::engine::pipeline::run_pipeline;
use crate::engine::preview::build_input_source;
use crate::error::RetromountError;
use crate::mount::session::MountSession;

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

    Err(RetromountError::LoadError(format!(
        "mount is not implemented yet for mountpoint: {}",
        mountpoint.display()
    )))
}
