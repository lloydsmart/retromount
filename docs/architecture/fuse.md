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

File reads are delegated through the existing VFS reader pipeline:

`open_vfs_file → Reader → read_at(offset, buffer)`

This keeps filesystem adapter code thin and reuses the existing VFS-backed reader stack rather than introducing FUSE-specific file decoding logic.

## Boundary Notes

The FUSE adapter must not:

- reimplement pipeline stages
- reinterpret normalized content
- make presentation or encoding decisions
- bypass VFS reader abstractions

The FUSE layer is an adapter over the already-materialized VFS, not an alternative execution path.

## Current Scope

Phase 4A provides:

- Linux-only FUSE mounting
- read-only filesystem access
- inode-based directory traversal
- file reads backed by the Retromount VFS reader layer

## Future Enhancements

Potential future improvements include:

- caching and read-performance optimisations
- improved mount diagnostics and logging
- additional platform adapter implementations
