# Consumer Views (Phase 4B)

Retromount supports multiple filesystem layouts over the same normalized content.

This is achieved through **presenters**, which control how content is projected into the Virtual File System (VFS) without modifying the underlying pipeline.

---

## Overview

The Retromount pipeline remains unchanged:

1. Input
2. Identify
3. Decode
4. Normalize
5. Present
6. Encode
7. VFS

The **presenter** is responsible only for how content is arranged into directories and hierarchy.

---

## Available Views

### Grouped (default)

The grouped view represents the current, library-style layout.

Characteristics:

* content grouped by platform and title
* multi-disc titles grouped into a directory
* playlists (e.g. `.m3u`) appear alongside discs

Example:

```
ps1/
  Final Fantasy VII/
    Final Fantasy VII (Disc 1).cue
    Final Fantasy VII (Disc 2).cue
    Final Fantasy VII.m3u
```

---

### Flat

The flat view removes grouping and exposes content at the root.

Characteristics:

* no platform directories
* no title directories
* files appear directly at the VFS root
* multi-disc titles emit disc files plus playlist at root

Example:

```
Final Fantasy VII (Disc 1).cue
Final Fantasy VII (Disc 2).cue
Final Fantasy VII.m3u
Super Mario World.sfc
```

---

## CLI Usage

Presenter selection is exposed via the `--view` flag:

```
retromount inspect <input> --view grouped
retromount inspect <input> --view flat

retromount mount <input> <mountpoint> --view grouped
retromount mount <input> <mountpoint> --view flat
```

Default:

```
--view grouped
```

---

## Behavioural Notes

### Naming vs Structure

* **Presenters control structure** (directories, grouping)
* **Encoders control filenames**

The flat presenter does not modify naming rules beyond ensuring valid filesystem entries.

---

### Non-Game Content

For non-game content (e.g. text files, images, archives):

* encoder-derived paths are reduced to **leaf filenames**
* this ensures compatibility with filesystem constraints

Example:

```
mixed/notes.txt → notes.txt
roms/snes/cover.jpg → cover.jpg.bin
```

---

### Multi-Disc Titles

Multi-disc games are handled consistently across views:

* each disc is emitted as an individual file
* a playlist (`.m3u`) is generated referencing all discs

Grouped:

```
Final Fantasy VII/
  Disc 1
  Disc 2
  playlist
```

Flat:

```
Final Fantasy VII (Disc 1).cue
Final Fantasy VII (Disc 2).cue
Final Fantasy VII.m3u
```

---

### Collision Handling

The flat view may expose filename collisions when multiple files share the same leaf name.

Collision handling and naming policy are **out of scope for Phase 4B** and will be addressed in Phase 4C.

---

## Design Intent

Phase 4B establishes that:

> The same normalized content can be presented as multiple filesystem layouts without modifying core pipeline stages.

This separation enables:

* alternative layouts for different consumers
* future presenter implementations
* plugin-based extensibility (Phase 4D)

---

## Status

Phase 4B is complete when:

* multiple presenters are implemented
* CLI selection is supported
* grouped and flat views produce distinct VFS layouts
* behaviour is validated via tests and runtime mounts
