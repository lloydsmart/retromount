# Retromount

**Retromount** is a modular FUSE-based filesystem that presents retro game archives in the formats emulators expect — without duplicating or converting your collection.

Instead of maintaining multiple copies of the same games in different formats, Retromount mounts **virtual views** of your library where files are translated on demand.

Example:

| Stored on disk | Exposed to emulator |
|----------------|---------------------|
| `game.chd`     | `game.iso`          |
| `rom.zip`      | `rom.sfc`           |

The original files remain unchanged; Retromount performs the translation dynamically through a modular plugin system.

---

## Why Retromount?

Retro collections are often stored in archival formats to save space or preserve integrity, but emulators frequently require different formats.

Typical workflows involve:

- batch converting files
- extracting archives
- maintaining duplicate libraries

Retromount solves this by acting as a **translation layer between storage and emulators**, allowing collections to remain in their preferred archival format while still appearing exactly how software expects.

---

## Key Features

- **FUSE-based virtual filesystem**
- **On-demand format translation**
- **Plugin-based translator architecture**
- **Multiple simultaneous views of the same library**
- **No duplicate storage required**

---

## Core Concepts

Retromount separates three concepts:

### Storage

Where the original files live.

Examples:

- CHD archives
- ZIP ROM sets
- raw disc images

### Translators

Modules that convert between formats.

Examples:

```
CHD → ISO
ZIP → ROM
BIN/CUE → ISO
```

### Views

A mounted filesystem that exposes translated files.

Example configuration:

```yaml
- name: ps2
  source: /roms/ps2_chd
  mount: /mnt/ps2_iso
  translator: chd_to_iso
```

This would allow:

```
/roms/ps2_chd/game.chd
```

to appear as:

```
/mnt/ps2_iso/game.iso
```

---

## Architecture

Retromount is designed around a **modular plugin architecture**.

```
           +---------------------+
           |  Emulator / Client  |
           +----------+----------+
                      |
                      v
                FUSE Filesystem
                      |
          +-----------+------------+
          |                        |
     Translators              Config Engine
          |
          v
     Source Storage
```

Future translators may include:

- CHD → ISO
- ZIP → ROM extraction
- archive passthrough
- format normalization
- emulator-specific views

---

## Status

⚠ **Early development**

Retromount is currently in the **initial scaffolding stage**.

Planned milestones:

1. Core configuration system
2. Basic FUSE filesystem
3. Translator plugin interface
4. First translator: **CHD → ISO**
5. Multi-view support
6. Performance optimisation

---

## Building

Requirements:

- Rust (stable)
- libfuse development libraries

Ubuntu / Debian:

```bash
sudo apt install libfuse-dev
```

Build:

```bash
cargo build
```

Run:

```bash
cargo run
```

---

## Example Configuration

`retromount.yaml`

```yaml
- name: ps2
  source: /roms/ps2_chd
  mount: /mnt/ps2_iso
  translator: chd_to_iso
```

---

## Goals

Retromount aims to:

- eliminate duplicate ROM storage
- support large retro collections
- integrate cleanly with emulators
- support multiple output formats simultaneously
- remain lightweight and fast

---

## License

GPL-3.0