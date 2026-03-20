use serde::Serialize;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VfsFile {
    pub name: String,
    pub size: u64,
    pub contents: Option<Vec<u8>>,
}

impl VfsFile {
    pub fn new(name: impl Into<String>, size: u64) -> Self {
        Self {
            name: name.into(),
            size,
            contents: None,
        }
    }

    pub fn with_contents(name: impl Into<String>, contents: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            size: contents.len() as u64,
            contents: Some(contents),
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
                vec![VfsNode::File(VfsFile::new("game.sfc", 1024))],
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
    fn builds_virtual_file_with_inline_contents() {
        let file = VfsFile::with_contents("game.m3u", b"game (Disc 1).cue\n".to_vec());

        assert_eq!(file.name, "game.m3u");
        assert_eq!(file.size, 18);
        assert_eq!(file.contents, Some(b"game (Disc 1).cue\n".to_vec()));
    }
}
