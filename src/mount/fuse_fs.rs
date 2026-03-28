use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use fuser::{
    mount2, Config, FileAttr, FileHandle, FileType, Filesystem, Generation, INodeNo, KernelConfig,
    ReplyAttr, ReplyDirectory, ReplyEntry, Request,
};

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
            MountNodeKind::File => FileType::RegularFile,
        };

        let perm = match kind {
            FileType::Directory => 0o555,
            FileType::RegularFile => 0o444,
            _ => 0o444,
        };

        let size = match node.kind {
            MountNodeKind::Directory { .. } => 0,
            MountNodeKind::File => 0,
        };

        FileAttr {
            ino: INodeNo(node.inode),
            size,
            blocks: 0,
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
}

impl FilesystemAdapter for RetromountFuseFs {
    fn mount(self, mountpoint: &Path) -> Result<(), RetromountError> {
        let config = Self::config();
        mount2(self, mountpoint, &config).map_err(|err| RetromountError::LoadError(err.to_string()))
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
                MountNodeKind::File => FileType::RegularFile,
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
