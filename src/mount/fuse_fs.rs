use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use fuser::{
    mount, Config, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    KernelConfig, LockOwner, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, Request,
};

use crate::core::vfs_reader::open_vfs_file;
use crate::error::RetromountError;
use crate::mount::adapter::FilesystemAdapter;
use crate::mount::session::{MountNode, MountNodeKind, MountSession};

const TTL: Duration = Duration::from_secs(1);

pub struct RetromountFuseFs {
    session: MountSession,
}

impl RetromountFuseFs {
    pub fn new(session: MountSession) -> Self {
        Self { session }
    }

    pub fn config() -> Config {
        Config::default()
    }

    fn file_attr_for_node(&self, node: &MountNode) -> FileAttr {
        let kind = match node.kind {
            MountNodeKind::Directory { .. } => FileType::Directory,
            MountNodeKind::File { .. } => FileType::RegularFile,
        };

        let perm = match kind {
            FileType::Directory => 0o555,
            FileType::RegularFile => 0o444,
            _ => 0o444,
        };

        let size = match &node.kind {
            MountNodeKind::Directory { .. } => 0,
            MountNodeKind::File { file } => file.size,
        };

        FileAttr {
            ino: INodeNo(node.inode),
            size,
            blocks: size.div_ceil(512),
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind,
            perm,
            nlink: match kind {
                FileType::Directory => 2,
                _ => 1,
            },
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    fn read_file_range(&self, ino: u64, offset: u64, size: u32) -> Result<Vec<u8>, fuser::Errno> {
        let Some(file) = self.session.file(ino) else {
            return Err(fuser::Errno::EISDIR);
        };

        if offset >= file.size {
            return Ok(Vec::new());
        }

        let mut reader = open_vfs_file(file).map_err(|_| fuser::Errno::EIO)?;
        let mut buf = vec![0; size as usize];

        let bytes_read = reader
            .read_at(offset, &mut buf)
            .map_err(|_| fuser::Errno::EIO)?;

        buf.truncate(bytes_read);
        Ok(buf)
    }
}

impl FilesystemAdapter for RetromountFuseFs {
    fn mount(self, mountpoint: &Path) -> Result<(), RetromountError> {
        let config = Self::config();
        mount(self, mountpoint, &config).map_err(|err| RetromountError::LoadError(err.to_string()))
    }
}

impl Filesystem for RetromountFuseFs {
    fn init(&mut self, _req: &Request, _config: &mut KernelConfig) -> io::Result<()> {
        Ok(())
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(name) = name.to_str() else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };

        let Some(node) = self.session.lookup_child(parent.0, name) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };

        let attr = self.file_attr_for_node(node);
        reply.entry(&TTL, &attr, Generation(0));
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let Some(node) = self.session.get(ino.0) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };

        let attr = self.file_attr_for_node(node);
        reply.attr(&TTL, &attr);
    }

    fn open(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        if self.session.file(ino.0).is_none() {
            reply.error(fuser::Errno::EISDIR);
            return;
        }

        reply.opened(FileHandle(ino.0), FopenFlags::empty());
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        match self.read_file_range(ino.0, offset, size) {
            Ok(data) => reply.data(&data),
            Err(err) => reply.error(err),
        };
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let Some(node) = self.session.get(ino.0) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };

        let MountNodeKind::Directory { .. } = &node.kind else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };

        let Some(children) = self.session.children(ino.0) else {
            reply.error(fuser::Errno::EIO);
            return;
        };

        let mut entries: Vec<(INodeNo, FileType, String)> = Vec::new();

        entries.push((ino, FileType::Directory, ".".to_string()));

        let parent_inode = node.parent_inode.unwrap_or(ino.0);
        entries.push((INodeNo(parent_inode), FileType::Directory, "..".to_string()));

        for child in children {
            let file_type = match child.kind {
                MountNodeKind::Directory { .. } => FileType::Directory,
                MountNodeKind::File { .. } => FileType::RegularFile,
            };

            entries.push((INodeNo(child.inode), file_type, child.name.clone()));
        }

        for (index, (entry_ino, file_type, name)) in
            entries.into_iter().enumerate().skip(offset as usize)
        {
            let full = reply.add(entry_ino, (index + 1) as u64, file_type, name);
            if full {
                break;
            }
        }

        reply.ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};

    fn inline_session() -> (MountSession, u64) {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::File(VfsFile::inline(
                "playlist.m3u",
                b"disc1.cue\ndisc2.cue\n".to_vec(),
            ))],
        );

        let session = MountSession::from_root(&root);
        let inode = session
            .lookup_child(session.root_inode(), "playlist.m3u")
            .expect("expected inline file")
            .inode;

        (session, inode)
    }

    #[test]
    fn read_file_range_reads_full_inline_file() {
        let (session, ino) = inline_session();
        let fs = RetromountFuseFs::new(session);

        let data = fs
            .read_file_range(ino, 0, 1024)
            .expect("expected read to succeed");

        assert_eq!(data, b"disc1.cue\ndisc2.cue\n");
    }

    #[test]
    fn read_file_range_reads_partial_inline_file() {
        let (session, ino) = inline_session();
        let fs = RetromountFuseFs::new(session);

        let data = fs
            .read_file_range(ino, 5, 8)
            .expect("expected partial read to succeed");

        assert_eq!(data, b".cue\ndis");
    }

    #[test]
    fn read_file_range_returns_empty_at_eof() {
        let (session, ino) = inline_session();
        let fs = RetromountFuseFs::new(session);

        let data = fs
            .read_file_range(ino, 20, 16)
            .expect("expected eof read to succeed");

        assert!(data.is_empty());
    }

    #[test]
    fn read_file_range_returns_error_for_directory_inode() {
        let (session, _) = inline_session();
        let fs = RetromountFuseFs::new(session);

        let result = fs.read_file_range(fs.session.root_inode(), 0, 16);

        assert!(result.is_err(), "expected directory read to fail");
    }

    #[test]
    fn file_attr_uses_backing_file_size() {
        let (session, ino) = inline_session();
        let fs = RetromountFuseFs::new(session);

        let node = fs.session.get(ino).expect("expected mounted node");
        let attr = fs.file_attr_for_node(node);

        assert_eq!(attr.size, b"disc1.cue\ndisc2.cue\n".len() as u64);
        assert_eq!(attr.kind, FileType::RegularFile);
    }
}
