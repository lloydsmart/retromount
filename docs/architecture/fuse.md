# FUSE Integration

## Overview

Phase 4A introduces a Linux FUSE adapter that exposes the Retromount VFS as a read-only filesystem.

The design maintains a strict separation between:

- core pipeline and VFS logic
- platform-specific filesystem adapters

## Key Components

### MountSession

Provides an indexed, inode-based representation of the VFS:

- stable inode allocation
- parent/child relationships
- efficient lookup and traversal

### RetromountFuseFs

Implements the `fuser::Filesystem` trait:

- `lookup`, `getattr`, `readdir` for directory traversal
- `open`, `read` for file access

### VFS Reader Integration

File reads are delegated to:

```text
open_vfs_file -> Reader -> read_at(offset, buffer)
