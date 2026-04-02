use crate::core::vfs::{VfsDirectory, VfsNode};
use crate::policy::PolicySet;

pub fn allocate_file_name(directory: &VfsDirectory, proposed: &str, policy: &PolicySet) -> String {
    let existing: Vec<String> = directory
        .children()
        .iter()
        .map(|node| node.name().to_string())
        .collect();

    policy.conflict().resolve_name_conflict(proposed, &existing)
}

pub fn allocate_directory_name(
    directory: &VfsDirectory,
    proposed: &str,
    policy: &PolicySet,
) -> String {
    let existing: Vec<String> = directory
        .children()
        .iter()
        .filter_map(|node| match node {
            VfsNode::Directory(dir) => Some(dir.name.clone()),
            _ => None,
        })
        .collect();

    if existing.iter().any(|name| name == proposed) {
        proposed.to_string()
    } else {
        policy.conflict().resolve_name_conflict(proposed, &existing)
    }
}
