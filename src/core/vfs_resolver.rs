use std::io;

use crate::core::reader::Reader;
use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};
use crate::core::vfs_reader::open_vfs_file;

pub fn find_node<'a>(root: &'a VfsDirectory, path: &str) -> Option<&'a VfsNode> {
    let path = normalize_vfs_path(path);

    if path.is_empty() {
        return None;
    }

    root.find_node(&path)
}

pub fn find_directory<'a>(root: &'a VfsDirectory, path: &str) -> Option<&'a VfsDirectory> {
    let path = normalize_vfs_path(path);

    if path.is_empty() {
        return Some(root);
    }

    root.find_directory(&path)
}

pub fn find_file<'a>(root: &'a VfsDirectory, path: &str) -> Option<&'a VfsFile> {
    let path = normalize_vfs_path(path);

    if path.is_empty() {
        return None;
    }

    root.find_file(&path)
}

pub fn open_file(root: &VfsDirectory, path: &str) -> Result<Box<dyn Reader>, io::Error> {
    let file = find_file(root, path).ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("file not found: {path}"))
    })?;

    open_vfs_file(file)
}

fn normalize_vfs_path(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::SourceRef;
    use crate::core::vfs::{VfsFile, VfsNode};

    #[test]
    fn finds_root_file_by_path() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::File(VfsFile::inline(
                "mixed/notes.txt",
                b"hello".to_vec(),
            ))],
        );

        let file = find_file(&root, "mixed/notes.txt").expect("file should exist");
        assert_eq!(file.name, "mixed/notes.txt");
    }

    #[test]
    fn finds_directory_by_path() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![VfsNode::File(VfsFile::inline(
                    "game.m3u",
                    b"disc1.cue\ndisc2.cue\n".to_vec(),
                ))],
            ))],
        );

        let dir = find_directory(&root, "game").expect("directory should exist");
        assert_eq!(dir.name, "game");
    }

    #[test]
    fn finds_root_directory_for_empty_path() {
        let root = VfsDirectory::with_children("", vec![]);

        let dir = find_directory(&root, "").expect("root directory should exist");
        assert_eq!(dir.name, "");
    }

    #[test]
    fn opens_inline_file_by_path() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "game",
                vec![VfsNode::File(VfsFile::inline(
                    "game.m3u",
                    b"disc1.cue\ndisc2.cue\n".to_vec(),
                ))],
            ))],
        );

        let mut reader = open_file(&root, "game/game.m3u").unwrap();
        let mut buf = vec![0; 20];
        let bytes = reader.read_at(0, &mut buf).unwrap();

        assert_eq!(&buf[..bytes], b"disc1.cue\ndisc2.cue\n");
    }

    #[test]
    fn opens_source_backed_file_by_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("readme.txt");
        std::fs::write(&path, b"hello world").unwrap();

        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::File(VfsFile::source_backed(
                "docs/readme.txt",
                11,
                SourceRef::new(path.to_string_lossy().into_owned()),
            ))],
        );

        let mut reader = open_file(&root, "docs/readme.txt").unwrap();
        let mut buf = vec![0; 11];
        let bytes = reader.read_at(0, &mut buf).unwrap();

        assert_eq!(bytes, 11);
        assert_eq!(&buf, b"hello world");
    }

    #[test]
    fn normalizes_backslashes_in_resolved_paths() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::File(VfsFile::inline(
                "mixed/notes.txt",
                b"hello".to_vec(),
            ))],
        );

        let file = find_file(&root, r"mixed\notes.txt").expect("file should exist");
        assert_eq!(file.name, "mixed/notes.txt");
    }

    #[test]
    fn returns_not_found_when_opening_missing_file() {
        let root = VfsDirectory::with_children("", vec![]);

        let err = open_file(&root, "missing.txt").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
