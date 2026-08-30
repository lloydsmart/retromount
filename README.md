# 🎮 RetroMount

![Rust](https://img.shields.io/badge/Rust-stable-orange)
[![CI](https://github.com/lloydsmart/retromount/actions/workflows/validate-rust.yml/badge.svg?branch=develop)](https://github.com/lloydsmart/retromount/actions/workflows/validate-rust.yml)
[![Rust Lint](https://github.com/lloydsmart/retromount/actions/workflows/lint-rust.yml/badge.svg?branch=develop)](https://github.com/lloydsmart/retromount/actions/workflows/lint-rust.yml)
[![Markdown Lint](https://github.com/lloydsmart/retromount/actions/workflows/lint-docs.yml/badge.svg?branch=develop)](https://github.com/lloydsmart/retromount/actions/workflows/lint-docs.yml)
[![Actions Lint](https://github.com/lloydsmart/retromount/actions/workflows/lint-actions.yml/badge.svg?branch=develop)](https://github.com/lloydsmart/retromount/actions/workflows/lint-actions.yml)
[![Shell Lint](https://github.com/lloydsmart/retromount/actions/workflows/lint-shell.yml/badge.svg?branch=develop)](https://github.com/lloydsmart/retromount/actions/workflows/lint-shell.yml)
[![CodeQL](https://github.com/lloydsmart/retromount/actions/workflows/scan-codeql-rust.yml/badge.svg?branch=develop)](https://github.com/lloydsmart/retromount/actions/workflows/scan-codeql-rust.yml)
[![License](https://img.shields.io/github/license/lloydsmart/retromount)](LICENSE)

> A virtual filesystem for retro game collections that can be mounted and transformed on-the-fly without duplicating files.

RetroMount is an experimental Rust project for building a **virtual filesystem over retro game collections**.

It allows ROMs, disc images, and archives to be **mounted into alternative filesystem layouts on-the-fly**, enabling different devices and emulators to view the same underlying collection in different formats without duplicating files.

Implemented examples include:

* Present supported PS2 DVD CHD/ISO inputs and PS2 CD ISO/CUE/BIN inputs as
  OPL-compatible ISO files
* Expose ROMs from ZIP archives through flat or grouped layouts
* Present supported PS1 CUE/BIN, ISO, and CHD inputs in a DuckStation layout
* Load custom presentation layouts from versioned YAML files

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
/roms/ps2/game.chd
```

RetroMount can present that file through its OPL presentation as:

```text
/mnt/opl/DVD/game.iso
```

All backed by **one underlying file**.

---

## Project Status

Retromount is under active development and progressing through a structured roadmap.

### Completed

* Phase 1 — Foundations
* Phase 2 — Core abstractions and initial pipeline
* Phase 3 — Pipeline consolidation and normalized content model
* Phase 4A — Mountable filesystem (FUSE integration)
* Phase 4B — Consumer views (multiple presentations)
* Phase 4C — Naming and conflict resolution policies
* Phase 4D — Extensibility and configuration layer
* Phase 5 — Runtime encoder plugin architecture
* Phase 6 — Declarative presentation specifications
* Versioned YAML presentation files and external presentation loading
* First practical consumer target — live PS2 DVD CHD to OPL-compatible ISO
  presentation
* First PS1 consumer target — the built-in `duckstation` presentation, with
  integration coverage for CUE/BIN, ISO, CHD, stored ZIP, multi-disc, and SBI
  inputs

### In Progress

* Optical-media capability expansion beyond the implemented OPL PS2 and
  DuckStation PS1 paths; materialized CHD encoding is deferred to caching work,
  and OPL/POPS integration still requires research

### Planned

* Additional optical-disc inputs and consumer presentations
* Performance, caching, and optimisation after representative input and media
  workloads are available
* Advanced encoders and external integrations (e.g. torrent compatibility)

---

## Getting Started (Linux)

### Prerequisites

You need Git, a C compiler/linker, and [Rustup](https://rustup.rs/). This
repository's `rust-toolchain.toml` selects the current stable Rust toolchain;
no minimum supported Rust version (MSRV) is declared.

To use the `mount` command, the Linux host must also provide kernel FUSE
support (`/dev/fuse`) and the `fusermount3` helper (or `fusermount` on a FUSE 2
system). On Debian or Ubuntu, install the required system packages with:

```bash
sudo apt-get update
sudo apt-get install build-essential fuse3 git
```

Other distributions need equivalent packages providing Git, a system linker,
and the FUSE runtime and mount helper. The current Linux build uses `fuser`'s
pure-Rust mount path, so libfuse development headers and `pkg-config` are not
required. If FUSE is installed but `/dev/fuse` is absent, load the kernel module
with `sudo modprobe fuse`.

### Clone and build

```bash
git clone https://github.com/lloydsmart/retromount.git
cd retromount
cargo build --locked
```

Rustup reads `rust-toolchain.toml` and installs stable Rust if necessary.

### Run a verified example

Inspect the tracked `LICENSE` file with the flat presentation:

```bash
cargo run --locked -- inspect LICENSE --presentation flat
```

This runs the complete pipeline without mounting anything. The report describes
the input, decoded and normalized content, and proposed virtual filesystem. Its
final section should be:

```text
Output VFS:
/
  LICENSE.bin
```

The `.bin` entry is the name Retromount would expose; the command does not write
a new `LICENSE.bin` file to the checkout.

---

## Mounting (Linux)

RetroMount can expose a file, ZIP archive, or directory as a read-only FUSE
filesystem. The mountpoint must already exist. For example, from the repository
root:

```bash
mkdir -p /tmp/retromount
cargo run --locked -- mount LICENSE /tmp/retromount --presentation flat
```

The Retromount process stays in the foreground while the filesystem is mounted.
In another terminal, inspect it and then unmount it:

```bash
ls /tmp/retromount
fusermount3 -u /tmp/retromount
```

The `ls` command should show `LICENSE.bin`. On a FUSE 2 system, use
`fusermount -u /tmp/retromount` instead. After unmounting, the foreground
Retromount process exits.

### Notes

* FUSE mounting is Linux only
* the mounted filesystem is read-only
* output reflects the same structure shown by `phase3-preview` and `inspect`
* performance optimisations are planned in future phases

---

## CLI Reference

During development, invoke commands with `cargo run --locked --` as shown
below. If you run an installed `retromount` binary, omit that prefix. The
current CLI parser is order-sensitive; when combining options, keep them in
the order shown.

### `mount`

```bash
cargo run --locked -- mount <input> <mountpoint> \
  [--presentation <name-or-yaml-file>] [--plugin-dir <dir>]
```

`mount` runs the pipeline for a regular file, ZIP archive, or directory, then
serves the resulting VFS at the existing mountpoint until it is unmounted. The
default presentation is `grouped`.

### `inspect`

```bash
cargo run --locked -- inspect <path> [--json] \
  [--presentation <name-or-yaml-file>] [--plugin-dir <dir>]
```

`inspect` runs the pipeline without mounting and reports the input objects,
decoding and normalization results, and output VFS. `--json` emits the pipeline
trace as JSON instead of the text report.

For `mount` and `inspect`, `--presentation` accepts the built-in names
`duckstation`, `flat`, `grouped`, and `opl`, or the path to a versioned `.yaml`
or `.yml` presentation file. The older `--view` spelling remains a compatibility
alias. `--plugin-dir` loads runtime encoder plugins from the supplied directory.
There are no CLI `--platform` or `--media` options; those hints are available
only through configuration.

### `phase3-preview`

```bash
cargo run --locked -- phase3-preview <path> [--plugin-dir <dir>]
```

`phase3-preview` runs the default `grouped` presentation and prints only the
proposed VFS tree. It accepts a regular file, ZIP archive, or directory. The
development-era command name is transitional, and this command does not
currently accept `--presentation` or `--json`.

---

## Example Configuration

Running `retromount` with no arguments reads `retromount.yaml` from the current
directory. For each configured view it builds the pipeline and logs the
resulting VFS tree at info level (`RUST_LOG=info`). This path does not mount the
tree at the configured `mount` path; use the explicit `retromount mount`
command to create a FUSE mount.

Create `retromount.yaml`:

```yaml
- name: ps1
  source: /roms/ps1/Ridge Racer.cue
  mount: /mnt/retromount/ps1
  platform: ps1
  presentation: grouped

- name: ps2-opl
  source: /roms/ps2/Test Game.chd
  mount: /mnt/retromount/ps2
  platform: ps2
  media: dvd
  presentation: opl

- name: snes
  source: /roms/snes
  mount: /mnt/retromount/snes
  platform: snes

- name: megadrive
  source: /roms/megadrive
  mount: /mnt/retromount/megadrive
  platform: megadrive
  presentation: flat
```

Fields:

| Field          | Description                                                          |
| -------------- | -------------------------------------------------------------------- |
| `name`         | Logical view name used in log output                                 |
| `source`       | Source directory, archive, or disc image                             |
| `mount`        | Required path that is currently logged but not mounted by this path  |
| `platform`     | Normalization hint (e.g. `ps1`, `ps2`, `snes`, `megadrive`)          |
| `media`        | Optional `cd`/`dvd` hint for ISO input in OPL or DuckStation         |
| `presentation` | Built-in name or YAML file path (default: `grouped`)                 |
| `encoder`      | Accepted by the configuration parser but currently unused            |

Platform names are **case-insensitive** and accept friendly aliases.

---

### Composition

Each configured view selects a **presentation specification**, which defines
filesystem structure and artifact requirements. Capability resolution then
selects the available encoders needed to materialize those artifacts; the
configured `encoder` field does not override that selection.

If not specified:

* `presentation` selects a built-in name or versioned YAML file and defaults
  to `grouped`
* the legacy `presenter` field remains a compatibility alias

This allows different views to present the same underlying data in different layouts without duplicating files.

---

## Architecture Overview

Retromount uses one processing model for inspection, previews, and mounts. Input
formats are interpreted before output choices are made, with a normalized
content model separating source details from consumer-specific filesystem
layouts.

```text
Input discovery and enumeration
              │
              ▼
Identification and format decoding
              │
              ▼
Normalized content model
              │
              ▼
Presentation compilation and planning
              │
              ▼
Artifact resolution and materialization
              │
              ▼
Read-only virtual filesystem (VFS)
              │
              └──► Optional Linux FUSE exposure
```

### Discovery and source interpretation

The input path first selects a source enumerator. A regular file produces one
object, a directory is walked recursively, and a ZIP archive is opened as a
container whose non-directory entries become objects. Each object retains the
metadata and byte access needed by later pipeline stages.
At this stage ZIP is a discovery concern: it determines what input objects are
available, not what those objects mean.

Each object is then identified and passed to a decoder that understands its
source format. CUE, CHD, and ISO semantics are handled here, including disc
layout and any safe track-aware or contiguous views of the media. Ordinary ROM,
text, and byte content follows the same identify-and-decode boundary. Discovery
therefore answers "what content is available?", while decoding answers "what
does this content represent?"

### Normalized content

Decoders preserve source-format structure, but downstream presentation does not
consume decoder-specific results directly. Normalization creates a shared,
presentation-agnostic model of games, ROMs, discs, text, and bytes. It applies
semantic information such as platform and disc order, groups related discs into
games, and prevents files consumed by compound inputs from also appearing as
independent content.

This boundary lets the same interpreted content feed different consumer views
without reparsing inputs or embedding filenames, directory layouts, or output
formats in the core model.

### Presentation planning

A command or configured view selects a built-in or versioned YAML presentation
before the run is composed. The presentation specification itself is applied
after normalization. It declares which normalized content to select, how to
arrange and name it, and which output artifacts and formats are required.
Current built-in consumer presentations may also provide platform or optical
media hints when the run is composed.

The presentation compiler combines that specification with naming and conflict
policy to produce a concrete plan of directories, files, multi-file artifact
sets, and generated items such as multi-disc playlists. The plan describes the
desired result and its artifact requirements; it does not choose or implement
an encoder.

### Artifact production and VFS construction

For every planned artifact, the host compares its required content type, format,
and features with the capabilities advertised by built-in and runtime-plugin
encoders. Resolution selects an encoder deterministically; that encoder then
materializes the requested representation. Materialization can preserve a
source-backed file, generate inline content, or provide a reader-backed view, so
it does not necessarily copy data into a separate output file.

The materialized entries become a read-only VFS tree. Inspection and preview
commands report that tree directly. On Linux, the mount path indexes the same
tree as filesystem nodes and gives it to a thin FUSE adapter, which handles
directory traversal and delegates file reads to the existing VFS reader layer.
FUSE does not repeat discovery, decoding, presentation, or encoding decisions.

For source navigation, orchestration lives in `src/engine/pipeline.rs`; input
enumeration and decoding in `src/input`; normalization, semantic content, and
VFS primitives in `src/core`; presentation planning, capability resolution, and
materialization in `src/output`; and FUSE integration in `src/mount`.

---

## Filtering

Discovery enumerates all regular files and ZIP entries; it does not
automatically exclude `.DS_Store`, `Thumbs.db`, `__MACOSX/`, or other names.

Presentation file rules can constrain normalized games with `source_formats`
and `excluded_source_formats`. The compiler derives each game part's format
from its source extension (`chd`, `iso`, `cue`, `bin`, or `zip`), and every part
must satisfy the rule. These constraints choose which presentation rule emits
an artifact; they do not filter discovery or convert the source. The built-in
`duckstation` presentation uses them to pass existing CHDs through natively
while requesting CUE/BIN artifacts for other supported PS1 sources.

---

## Documentation

* [Consumer Views (Phase 4B)](docs/consumer-views.md)
* [Presentation files](docs/presentations.md)
* [Contributing](CONTRIBUTING.md)
* [Security policy](SECURITY.md)

---

## Roadmap

### Phase 4 (completed)

* Mountable virtual filesystem (FUSE)
* Multiple consumer views
* Presentation policies (naming, conflict resolution)
* Explicit presentation configuration

### Phase 6 (completed)

* Declarative `PresentationSpec` model
* Generic presentation compiler
* Flat and grouped layout parity
* Multi-disc playlists and preserved relative paths
* Spec-native `phase3-preview`, `inspect`, `mount`, and plugin execution
* Legacy presenter implementations retired

---

## License

This project is licensed under the [GNU General Public License v3.0 only](LICENSE).
