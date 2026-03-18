#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsFile {
    pub name: String,
    pub size: u64,
}

impl VfsFile {
    pub fn new(name: impl Into<String>, size: u64) -> Self {
        Self {
            name: name.into(),
            size,
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
}
