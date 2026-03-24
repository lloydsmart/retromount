use serde::Serialize;

use crate::core::source::SourceRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum VfsNode {
    Directory(VfsDirectory),
    File(VfsFile),
}

impl VfsNode {
    pub fn name(&self) -> &str {
        match self {
            Self::Directory(dir) => &dir.name,
            Self::File(file) => &file.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VfsDirectory {
    pub name: String,
    pub children: Vec<VfsNode>,
}

impl VfsDirectory {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            children: Vec::new(),
        }
    }

    pub fn with_children(name: impl Into<String>, mut children: Vec<VfsNode>) -> Self {
        sort_nodes(&mut children);

        Self {
            name: name.into(),
            children,
        }
    }

    pub fn add_child(&mut self, child: VfsNode) {
        self.children.push(child);
        sort_nodes(&mut self.children);
    }

    pub fn children(&self) -> &[VfsNode] {
        &self.children
    }

    pub fn files(&self) -> impl Iterator<Item = &VfsFile> {
        self.children.iter().filter_map(|node| match node {
            VfsNode::File(file) => Some(file),
            VfsNode::Directory(_) => None,
        })
    }

    pub fn directories(&self) -> impl Iterator<Item = &VfsDirectory> {
        self.children.iter().filter_map(|node| match node {
            VfsNode::Directory(dir) => Some(dir),
            VfsNode::File(_) => None,
        })
    }

    pub fn find_node(&self, path: &str) -> Option<&VfsNode> {
        let path = normalize_vfs_path(path);

        if path.is_empty() {
            return None;
        }

        let (head, remainder) = split_path(&path);

        for child in &self.children {
            match child {
                VfsNode::File(file) => {
                    if remainder.is_none() && file.name == head {
                        return Some(child);
                    }

                    if file.name == path {
                        return Some(child);
                    }
                }
                VfsNode::Directory(dir) => {
                    if dir.name == head {
                        match remainder {
                            Some(remainder) => {
                                if let Some(node) = dir.find_node(remainder) {
                                    return Some(node);
                                }
                            }
                            None => return Some(child),
                        }
                    }
                }
            }
        }

        None
    }

    pub fn find_file(&self, path: &str) -> Option<&VfsFile> {
        match self.find_node(path) {
            Some(VfsNode::File(file)) => Some(file),
            _ => None,
        }
    }

    pub fn find_directory(&self, path: &str) -> Option<&VfsDirectory> {
        let path = normalize_vfs_path(path);

        if path.is_empty() {
            return Some(self);
        }

        match self.find_node(&path) {
            Some(VfsNode::Directory(dir)) => Some(dir),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FileBacking {
    Source(SourceRef),
    Inline(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VfsFile {
    pub name: String,
    pub size: u64,
    pub backing: FileBacking,
}

impl VfsFile {
    pub fn new(name: impl Into<String>) -> Self {
        Self::inline(name, Vec::new())
    }

    pub fn source_backed(name: impl Into<String>, size: u64, source: SourceRef) -> Self {
        Self {
            name: name.into(),
            size,
            backing: FileBacking::Source(source),
        }
    }

    pub fn inline(name: impl Into<String>, contents: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            size: contents.len() as u64,
            backing: FileBacking::Inline(contents),
        }
    }
}

fn normalize_vfs_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn split_path(path: &str) -> (&str, Option<&str>) {
    match path.split_once('/') {
        Some((head, tail)) => (head, Some(tail)),
        None => (path, None),
    }
}

fn sort_nodes(nodes: &mut [VfsNode]) {
    nodes.sort_by(|left, right| {
        node_kind_order(left)
            .cmp(&node_kind_order(right))
            .then_with(|| {
                left.name()
                    .to_ascii_lowercase()
                    .cmp(&right.name().to_ascii_lowercase())
            })
            .then_with(|| left.name().cmp(right.name()))
    });
}

fn node_kind_order(node: &VfsNode) -> u8 {
    match node {
        VfsNode::Directory(_) => 0,
        VfsNode::File(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_simple_tree() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "snes",
                vec![VfsNode::File(VfsFile::new("game.sfc"))],
            ))],
        );

        assert_eq!(root.children().len(), 1);

        match &root.children()[0] {
            VfsNode::Directory(dir) => {
                assert_eq!(dir.name, "snes");
                assert_eq!(dir.children().len(), 1);
            }
            _ => panic!("expected directory"),
        }
    }

    #[test]
    fn builds_inline_file() {
        let file = VfsFile::inline("game.m3u", b"game (Disc 1).cue\n".to_vec());

        assert_eq!(file.name, "game.m3u");
        assert_eq!(file.size, 18);

        match &file.backing {
            FileBacking::Inline(contents) => {
                assert_eq!(contents, b"game (Disc 1).cue\n");
            }
            _ => panic!("expected inline backing"),
        }
    }

    #[test]
    fn finds_root_file_with_slash_in_name() {
        let root =
            VfsDirectory::with_children("", vec![VfsNode::File(VfsFile::new("mixed/notes.txt"))]);

        let file = root
            .find_file("mixed/notes.txt")
            .expect("file should exist");
        assert_eq!(file.name, "mixed/notes.txt");
    }

    #[test]
    fn finds_nested_file_in_directory() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![VfsNode::File(VfsFile::inline(
                    "game.m3u",
                    b"game (Disc 1).cue\ngame (Disc 2).cue\n".to_vec(),
                ))],
            ))],
        );

        let file = root.find_file("game/game.m3u").expect("file should exist");
        assert_eq!(file.name, "game.m3u");
    }

    #[test]
    fn finds_directory_by_path() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![VfsNode::File(VfsFile::inline(
                    "game.m3u",
                    b"game (Disc 1).cue\ngame (Disc 2).cue\n".to_vec(),
                ))],
            ))],
        );

        let dir = root.find_directory("game").expect("directory should exist");
        assert_eq!(dir.name, "game");
    }

    #[test]
    fn finds_root_directory_for_empty_path() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![],
            ))],
        );

        let dir = root
            .find_directory("")
            .expect("root directory should exist");
        assert_eq!(dir.name, "");
    }

    #[test]
    fn finds_node_for_root_file() {
        let root =
            VfsDirectory::with_children("", vec![VfsNode::File(VfsFile::new("mixed/notes.txt"))]);

        let node = root
            .find_node("mixed/notes.txt")
            .expect("node should exist");

        match node {
            VfsNode::File(file) => assert_eq!(file.name, "mixed/notes.txt"),
            other => panic!("expected file node, got {other:?}"),
        }
    }

    #[test]
    fn finds_node_for_directory() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![VfsNode::File(VfsFile::inline(
                    "game.m3u",
                    b"game (Disc 1).cue\ngame (Disc 2).cue\n".to_vec(),
                ))],
            ))],
        );

        let node = root.find_node("game").expect("node should exist");

        match node {
            VfsNode::Directory(dir) => assert_eq!(dir.name, "game"),
            other => panic!("expected directory node, got {other:?}"),
        }
    }

    #[test]
    fn normalizes_repeated_slashes_in_paths() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![VfsNode::File(VfsFile::inline(
                    "game.m3u",
                    b"game (Disc 1).cue\ngame (Disc 2).cue\n".to_vec(),
                ))],
            ))],
        );

        let file = root
            .find_file("//game///game.m3u")
            .expect("file should exist");
        assert_eq!(file.name, "game.m3u");
    }

    #[test]
    fn exposes_directory_browse_helpers() {
        let root = VfsDirectory::with_children(
            "",
            vec![
                VfsNode::Directory(VfsDirectory::with_children("a-dir", vec![])),
                VfsNode::File(VfsFile::new("b-file.txt")),
            ],
        );

        assert_eq!(root.children().len(), 2);
        assert_eq!(root.directories().count(), 1);
        assert_eq!(root.files().count(), 1);
    }

    #[test]
    fn sorts_directories_before_files_case_insensitively() {
        let root = VfsDirectory::with_children(
            "",
            vec![
                VfsNode::File(VfsFile::new("z-file.txt")),
                VfsNode::Directory(VfsDirectory::with_children("b-dir", vec![])),
                VfsNode::File(VfsFile::new("a-file.txt")),
                VfsNode::Directory(VfsDirectory::with_children("a-dir", vec![])),
            ],
        );

        let names: Vec<_> = root.children().iter().map(|node| node.name()).collect();

        assert_eq!(names, vec!["a-dir", "b-dir", "a-file.txt", "z-file.txt"]);
    }

    #[test]
    fn add_child_maintains_stable_ordering() {
        let mut root = VfsDirectory::new("");

        root.add_child(VfsNode::File(VfsFile::new("z-file.txt")));
        root.add_child(VfsNode::Directory(VfsDirectory::with_children(
            "b-dir",
            vec![],
        )));
        root.add_child(VfsNode::File(VfsFile::new("a-file.txt")));
        root.add_child(VfsNode::Directory(VfsDirectory::with_children(
            "a-dir",
            vec![],
        )));

        let names: Vec<_> = root.children().iter().map(|node| node.name()).collect();

        assert_eq!(names, vec!["a-dir", "b-dir", "a-file.txt", "z-file.txt"]);
    }

    #[test]
    fn sorts_names_case_insensitively_for_browse_order() {
        let root = VfsDirectory::with_children(
            "",
            vec![
                VfsNode::File(VfsFile::new("zeta.txt")),
                VfsNode::File(VfsFile::new("Alpha.txt")),
                VfsNode::File(VfsFile::new("beta.txt")),
            ],
        );

        let names: Vec<_> = root.children().iter().map(|node| node.name()).collect();

        assert_eq!(names, vec!["Alpha.txt", "beta.txt", "zeta.txt"]);
    }
}
