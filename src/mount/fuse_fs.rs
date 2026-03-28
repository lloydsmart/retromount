use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

use fuser::{
    FileAttr, FileType, Filesystem, KernelConfig, MountOption, ReplyAttr, ReplyDirectory,
    ReplyEntry, Request,
};
use libc::{EIO, ENOENT};

use crate::mount::session::{MountNodeKind, MountSession};

const TTL: Duration = Duration::from_secs(1);

pub struct RetromountFuseFs {
    session: MountSession,
}

impl RetromountFuseFs {
    pub fn new(session: MountSession) -> Self {
        Self { session }
    }

    pub fn mount_options() -> Vec<MountOption> {
        vec![
            MountOption::RO,
            MountOption::FSName("retromount".to_string()),
            MountOption::DefaultPermissions,
        ]
    }

    fn file_attr_for_node(&self, node: &crate::mount::session::MountNode) -> FileAttr {
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
            ino: node.inode,
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

impl Filesystem for RetromountFuseFs {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), libc::c_int> {
        Ok(())
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let Some(name) = name.to_str() else {
            reply.error(ENOENT);
            return;
        };

        let Some(node) = self.session.lookup_child(parent, name) else {
            reply.error(ENOENT);
            return;
        };

        let attr = self.file_attr_for_node(node);
        reply.entry(&TTL, &attr, 0);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let Some(node) = self.session.get(ino) else {
            reply.error(ENOENT);
            return;
        };

        let attr = self.file_attr_for_node(node);
        reply.attr(&TTL, &attr);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let Some(node) = self.session.get(ino) else {
            reply.error(ENOENT);
            return;
        };

        let MountNodeKind::Directory { .. } = &node.kind else {
            reply.error(ENOENT);
            return;
        };

        let Some(children) = self.session.children(ino) else {
            reply.error(EIO);
            return;
        };

        let mut entries: Vec<(u64, FileType, String)> = Vec::new();

        entries.push((ino, FileType::Directory, ".".to_string()));

        let parent_inode = node.parent_inode.unwrap_or(ino);
        entries.push((parent_inode, FileType::Directory, "..".to_string()));

        for child in children {
            let file_type = match child.kind {
                MountNodeKind::Directory { .. } => FileType::Directory,
                MountNodeKind::File => FileType::RegularFile,
            };

            entries.push((child.inode, file_type, child.name.clone()));
        }

        for (index, (entry_ino, file_type, name)) in
            entries.into_iter().enumerate().skip(offset as usize)
        {
            let full = reply.add(entry_ino, (index + 1) as i64, file_type, name);
            if full {
                break;
            }
        }

        reply.ok();
    }
}
