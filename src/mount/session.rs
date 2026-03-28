use std::collections::{BTreeMap, HashMap};

use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};

#[derive(Debug, Clone)]
pub struct MountSession {
    root_inode: u64,
    nodes: HashMap<u64, MountNode>,
}

#[derive(Debug, Clone)]
pub struct MountNode {
    pub inode: u64,
    pub parent_inode: Option<u64>,
    pub name: String,
    pub kind: MountNodeKind,
}

#[derive(Debug, Clone)]
pub enum MountNodeKind {
    Directory { children: BTreeMap<String, u64> },
    File,
}

impl MountSession {
    pub fn from_root(root: &VfsDirectory) -> Self {
        let mut builder = MountSessionBuilder::new();
        let root_inode = builder.index_root(root);

        Self {
            root_inode,
            nodes: builder.nodes,
        }
    }

    pub fn root_inode(&self) -> u64 {
        self.root_inode
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, inode: u64) -> Option<&MountNode> {
        self.nodes.get(&inode)
    }

    pub fn lookup_child(&self, parent_inode: u64, name: &str) -> Option<&MountNode> {
        let parent = self.nodes.get(&parent_inode)?;

        match &parent.kind {
            MountNodeKind::Directory { children } => {
                let child_inode = children.get(name)?;
                self.nodes.get(child_inode)
            }
            MountNodeKind::File => None,
        }
    }

    pub fn children(&self, inode: u64) -> Option<Vec<&MountNode>> {
        let node = self.nodes.get(&inode)?;

        match &node.kind {
            MountNodeKind::Directory { children } => {
                let result = children
                    .values()
                    .filter_map(|child_inode| self.nodes.get(child_inode))
                    .collect();

                Some(result)
            }
            MountNodeKind::File => None,
        }
    }
}

struct MountSessionBuilder {
    next_inode: u64,
    nodes: HashMap<u64, MountNode>,
}

impl MountSessionBuilder {
    fn new() -> Self {
        Self {
            next_inode: 1,
            nodes: HashMap::new(),
        }
    }

    fn allocate_inode(&mut self) -> u64 {
        let inode = self.next_inode;
        self.next_inode += 1;
        inode
    }

    fn index_root(&mut self, root: &VfsDirectory) -> u64 {
        let root_inode = self.allocate_inode();
        let children = self.index_directory_children(root, root_inode);

        self.nodes.insert(
            root_inode,
            MountNode {
                inode: root_inode,
                parent_inode: None,
                name: "/".to_string(),
                kind: MountNodeKind::Directory { children },
            },
        );

        root_inode
    }

    fn index_directory(&mut self, directory: &VfsDirectory, parent_inode: u64) -> u64 {
        let inode = self.allocate_inode();
        let children = self.index_directory_children(directory, inode);

        self.nodes.insert(
            inode,
            MountNode {
                inode,
                parent_inode: Some(parent_inode),
                name: directory.name.clone(),
                kind: MountNodeKind::Directory { children },
            },
        );

        inode
    }

    fn index_file(&mut self, file: &VfsFile, parent_inode: u64) -> u64 {
        let inode = self.allocate_inode();

        self.nodes.insert(
            inode,
            MountNode {
                inode,
                parent_inode: Some(parent_inode),
                name: file.name.clone(),
                kind: MountNodeKind::File,
            },
        );

        inode
    }

    fn index_directory_children(
        &mut self,
        directory: &VfsDirectory,
        parent_inode: u64,
    ) -> BTreeMap<String, u64> {
        let mut children = BTreeMap::new();

        for child in directory.children() {
            match child {
                VfsNode::Directory(dir) => {
                    let inode = self.index_directory(dir, parent_inode);
                    children.insert(dir.name.clone(), inode);
                }
                VfsNode::File(file) => {
                    let inode = self.index_file(file, parent_inode);
                    children.insert(file.name.clone(), inode);
                }
            }
        }

        children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_root() -> VfsDirectory {
        VfsDirectory::with_children(
            "",
            vec![
                VfsNode::Directory(VfsDirectory::with_children(
                    "Example Game",
                    vec![
                        VfsNode::File(VfsFile::new("disc1.chd")),
                        VfsNode::File(VfsFile::new("game.m3u")),
                    ],
                )),
                VfsNode::File(VfsFile::new("readme.txt")),
            ],
        )
    }

    #[test]
    fn root_inode_is_one() {
        let root = sample_root();
        let session = MountSession::from_root(&root);

        assert_eq!(session.root_inode(), 1);
    }

    #[test]
    fn can_lookup_child_by_name() {
        let root = VfsDirectory::with_children("", vec![VfsNode::File(VfsFile::new("game.rom"))]);
        let session = MountSession::from_root(&root);

        let child = session
            .lookup_child(session.root_inode(), "game.rom")
            .expect("expected child node");

        assert_eq!(child.name, "game.rom");
        assert!(matches!(child.kind, MountNodeKind::File));
    }

    #[test]
    fn indexes_nested_directories() {
        let disc_dir = VfsDirectory::with_children(
            "Example Game",
            vec![VfsNode::File(VfsFile::new("disc1.chd"))],
        );

        let root = VfsDirectory::with_children("", vec![VfsNode::Directory(disc_dir)]);
        let session = MountSession::from_root(&root);

        let game_dir = session
            .lookup_child(session.root_inode(), "Example Game")
            .expect("expected game directory");

        assert!(matches!(game_dir.kind, MountNodeKind::Directory { .. }));

        let disc = session
            .lookup_child(game_dir.inode, "disc1.chd")
            .expect("expected disc file");

        assert_eq!(disc.parent_inode, Some(game_dir.inode));
        assert!(matches!(disc.kind, MountNodeKind::File));
    }

    #[test]
    fn node_count_covers_root_and_all_descendants() {
        let root = sample_root();
        let session = MountSession::from_root(&root);

        assert_eq!(session.node_count(), 5);
    }

    #[test]
    fn children_returns_directory_entries() {
        let root = sample_root();
        let session = MountSession::from_root(&root);

        let names: Vec<_> = session
            .children(session.root_inode())
            .expect("root should have children")
            .into_iter()
            .map(|node| node.name.as_str())
            .collect();

        assert_eq!(names, vec!["Example Game", "readme.txt"]);
    }

    #[test]
    fn lookup_child_returns_none_for_missing_entry() {
        let root = sample_root();
        let session = MountSession::from_root(&root);

        let missing = session.lookup_child(session.root_inode(), "does-not-exist.txt");
        assert!(missing.is_none());
    }

    #[test]
    fn children_returns_none_for_file_inode() {
        let root = sample_root();
        let session = MountSession::from_root(&root);

        let file = session
            .lookup_child(session.root_inode(), "readme.txt")
            .expect("expected root file");

        assert!(session.children(file.inode).is_none());
    }

    #[test]
    fn nested_nodes_have_correct_parent_inode() {
        let root = sample_root();
        let session = MountSession::from_root(&root);

        let game_dir = session
            .lookup_child(session.root_inode(), "Example Game")
            .expect("expected game directory");

        let playlist = session
            .lookup_child(game_dir.inode, "game.m3u")
            .expect("expected playlist file");

        assert_eq!(game_dir.parent_inode, Some(session.root_inode()));
        assert_eq!(playlist.parent_inode, Some(game_dir.inode));
    }
}
