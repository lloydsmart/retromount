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

    pub fn with_children(name: impl Into<String>, children: Vec<VfsNode>) -> Self {
        Self {
            name: name.into(),
            children,
        }
    }

    pub fn find_node(&self, path: &str) -> Option<&VfsNode> {
        let path = path.trim_matches('/');

        if path.is_empty() {
            return None;
        }

        for child in &self.children {
            match child {
                VfsNode::File(file) if file.name == path => return Some(child),
                VfsNode::Directory(dir) if dir.name == path => return Some(child),
                VfsNode::Directory(dir) => {
                    if let Some(remainder) = path.strip_prefix(&dir.name) {
                        if let Some(remainder) = remainder.strip_prefix('/') {
                            if let Some(node) = dir.find_node(remainder) {
                                return Some(node);
                            }
                        }
                    }
                }
                _ => {}
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
        match self.find_node(path) {
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

        assert_eq!(root.children.len(), 1);

        match &root.children[0] {
            VfsNode::Directory(dir) => {
                assert_eq!(dir.name, "snes");
                assert_eq!(dir.children.len(), 1);
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
}
