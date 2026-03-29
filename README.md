# 🎮 RetroMount

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

> A virtual filesystem for retro game collections that can be mounted and transformed on-the-fly without duplicating files.

RetroMount is an experimental Rust project for building a **virtual filesystem over retro game collections**.

It allows ROMs, disc images, and archives to be **mounted into alternative filesystem layouts on-the-fly**, enabling different devices and emulators to view the same underlying collection in different formats without duplicating files.

For example:

* Present CHD images as ISO files
* Extract ROMs from ZIP archives transparently
* Convert disc images into emulator-friendly layouts
* Provide platform-specific views for systems like MiSTer or Batocera

The long-term goal is a flexible **input → transformation → output pipeline** backed by a FUSE filesystem.

---

## What Problem Does RetroMount Solve?

Retro game collections are often stored in formats that are not ideal for every device or emulator.

For example:

| Storage format | Works well for               | Problems                                |
| -------------- | ---------------------------- | --------------------------------------- |
| ZIP archives   | ROM management               | Many emulators cannot read zipped files |
| CHD images     | Space-efficient storage      | Some tools require ISO/BIN files        |
| CUE/BIN discs  | Accurate disc representation | Some systems prefer CHD                 |
| Raw ROM sets   | Simple emulators             | Hard to manage at scale                 |

Traditionally, users solve this by **duplicating their collection in multiple formats**:

```text
ROMs/
  snes/
    game.zip
    game.sfc
```

or

```text
PS1/
  game.chd
  game.iso
```

This wastes storage space and creates maintenance headaches.

RetroMount solves this by providing a **virtual filesystem layer** that can dynamically transform game data into the format required by the target system.

Example:

```text
Storage (single copy)
    │
    ▼
/roms/ps1/game.chd
```

RetroMount can present that same file as:

```text
/mnt/mister/ps1/game.cue
/mnt/batocera/ps1/game.chd
/mnt/tools/ps1/game.iso
```

All backed by **one underlying file**.

---

## Project Status

RetroMount is under active development.

Current development is focused on exposing the pipeline through a real filesystem interface and refining presentation behaviour.

### Completed (Phase 4A)

* Linux FUSE mount support
* Read-only virtual filesystem backed by the VFS
* Directory traversal (`ls`, `tree`, `find`)
* File access (`cat`, `head`, etc.)
* Support for both inline-generated and source-backed files
* Multi-disc grouping with automatic playlist (`.m3u`) generation

### Completed (Phase 2)

The project now supports:

#### Input discovery

* Directory sources
* ZIP archives
* Individual files
* CUE/BIN disc images

#### Core models

* `VirtualFile` abstraction for discovered files
* `Track`, `Disc`, and `GameImage` models for disc-based systems

#### Loader pipeline

* Input handler registry
* Reader abstraction for accessing underlying data
* Loader for discovering payload files or loading disc images

#### CUE support

* Parsing CUE sheets
* Resolving referenced files
* Track ordering and deduplication
* Track size hydration from actual file metadata
* Generation of `GameImage` objects for disc-based systems

#### Configuration

* YAML configuration file
* Platform-aware views
* Flexible platform parsing (`ps1`, `playstation`, etc.)

---

## Mounting (Linux)

RetroMount can expose a collection as a read-only filesystem using FUSE.

```bash
retromount mount <input> <mountpoint>
```

### Example

```bash
retromount mount ./retromount-testdata /tmp/retromount-test
```

You can then browse and read files normally:

```bash
ls /tmp/retromount-test
tree /tmp/retromount-test
cat /tmp/retromount-test/ps1/Final\ Fantasy\ VII/Final\ Fantasy\ VII.m3u
head /tmp/retromount-test/snes/Super\ Mario\ World/Super\ Mario\ World.sfc
```

### Notes

* Linux only (via FUSE)
* filesystem is read-only
* output reflects the same structure as `preview` / `inspect`
* performance optimisations are planned in future phases

---

## Example Configuration

Create a file called `retromount.yaml`:

```yaml
- name: ps1
  source: /roms/ps1/Ridge Racer.cue
  mount: /mnt/retromount/ps1
  platform: ps1

- name: snes
  source: /roms/snes
  mount: /mnt/retromount/snes
  platform: snes

- name: megadrive
  source: /roms/megadrive
  mount: /mnt/retromount/megadrive
  platform: megadrive
```

Fields:

| Field      | Description                                       |
| ---------- | ------------------------------------------------- |
| `name`     | Logical name for the mounted view                 |
| `source`   | Source directory, archive, or disc image          |
| `mount`    | Mount point for the virtual filesystem            |
| `platform` | Target platform (e.g. `ps1`, `snes`, `megadrive`) |

Platform names are **case-insensitive** and accept friendly aliases.

---

## Architecture Overview

The ingestion pipeline currently looks like this:

```text
Input Source
     │
     ▼
InputHandler
     │
     ▼
InputRegistry
     │
     ▼
Loader
     │
 ┌───┴───────────────┐
 │                   │
 ▼                   ▼
VirtualFile list     GameImage
(file-oriented)      (disc-oriented)
```

### Input handlers

Input handlers are responsible for discovering files from a source.

Examples:

* `DirectoryInputHandler`
* `ZipInputHandler`
* `FileInputHandler`
* `CueInputHandler`

### Readers

Readers provide access to underlying data streams:

* `DirReader`
* `ZipReader`

### Core disc models

Disc-based systems use structured models:

```text
GameImage
 └─ Disc
     └─ Track
```

This enables accurate representation of multi-track disc formats.

---

## Filtering

RetroMount only removes **universally unwanted junk files** during discovery:

* `__MACOSX/`
* `.DS_Store`
* `Thumbs.db`

Other sidecar files such as `.nfo`, `.txt`, or cover art are preserved.

Output-specific filtering will be implemented in the **view/output layer** in a later phase.

---

## Roadmap

### Phase 4 (current)

* Consumer-specific filesystem views
* Presentation policies (naming, filtering, grouping)
* Performance improvements (e.g. read caching)
* Output translators (e.g. CHD → ISO)

---

## License

[GPL-3.0](LICENSE)
