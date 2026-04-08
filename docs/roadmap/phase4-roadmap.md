# Phase 4 Roadmap

This document defines the planned evolution of Retromount following completion of Phase 3 and the architecture/plugin readiness reviews.

It is intentionally concise and focused on execution.

---

## Scope

Retromount is a filesystem-oriented transformation and presentation layer for retro content collections.

It:

* normalizes existing content
* reorganizes it
* exposes it for different consumers

It does **not**:

* scrape metadata
* download artwork or assets
* manage ROM libraries
* replace ROM managers

> Retromount projects collections. It does not curate them.

Phase 4 introduces real filesystem exposure of these projections.

---

## Phase 4A — Mountable Filesystem

### Phase 4A Goal

Expose the Virtual File System (VFS) as a real, read-only filesystem via FUSE.

#### Status: Implemented — Validation in Progress

---

### Phase 4A Branch

feature/phase4a-fuse-mount

---

### Phase 4A Success Criteria

* [x] `retromount mount <input> <mountpoint>` works
* [x] directories can be browsed (`ls`, `tree`, `find`)
* [x] files can be read (`cat`, `head`, emulator, etc.)
* [x] output matches `preview` / `inspect`
* [x] multi-disc handling and playlists behave correctly
* [x] filesystem is read-only
* [x] no FUSE-specific logic leaks into core abstractions
* [ ] validated with at least one real consumer (e.g. emulator)

---

### Phase 4A Work Items

* CLI mount command
* FUSE adapter layer
* Mount session (inode/path index)
* Directory operations (`lookup`, `readdir`, `getattr`)
* File operations (`open`, `read`)
* Real-world validation

---

### Phase 4A Implementation Order

1. Mount command scaffold
2. Mount session / indexing layer
3. Directory traversal
4. File read support
5. Validation and hardening

---

### Phase 4A Commit Slices

* feat(cli): add mount command scaffold
* feat(mount): add mount session index for VFS nodes
* feat(fuse): implement read-only directory traversal
* feat(fuse): implement regular file reads
* test(mount): add VFS indexing coverage
* docs: document phase4a mount scope

---

### Phase 4A Progress Checklist

* [x] mount command added
* [x] FUSE adapter created
* [x] mount session/index implemented
* [x] directory traversal works
* [x] file reads work
* [x] tested with shell tools
* [ ] validated with a real consumer (e.g. emulator or tooling)
* [x] core refactors kept separate from adapter glue
* [x] documentation updated

---

### Phase 4A Validation

Validated on a real Linux host:

* mounted filesystem is browsable (`tree`, `find`)
* file metadata reports correct sizes
* generated inline files (e.g. `.m3u`) are readable
* source-backed files (e.g. ROMs, disc images) are readable
* repeated reads succeed consistently
* directory reads fail as expected

Outstanding validation:

* real consumer interaction (e.g. emulator loading content)
* verification of behaviour under sustained or repeated access patterns

---

### Phase 4A Notes

* This phase is both implementation and validation
* If architectural gaps are discovered, fix them in core rather than working around them in the adapter
* Keep scope tightly limited to read-only filesystem support
* current implementation reopens files on each `read()` call
* performance optimisations (reader caching, handle reuse) are explicitly deferred to a later phase

---

## Phase 4B — Consumer Views

### Phase 4B Goal

Support multiple filesystem layouts for different consumers without changing core normalization.

#### Status: Implemented

---

### Phase 4B Branch

feature/phase4b-consumer-views

---

### Phase 4B Success Criteria

* [x] Multiple presentation strategies supported
* [x] Same input can produce different layouts
* [x] Core pipeline remains unchanged
* [x] CLI selection of presenter via `--view`
* [x] `inspect` and `mount` both support presenter selection
* [x] At least two layouts implemented (`grouped`, `flat`)
* [x] Behaviour validated via tests and real mount

---

### Phase 4B Work Items

* [x] introduce presenter selection in runtime/CLI
* [x] formalize existing layout as `GroupedPresenter`
* [x] implement `FlatPresenter`
* [x] ensure multi-disc handling works across presenters
* [x] ensure encoder reuse across presenters
* [x] validate behaviour via `inspect` and FUSE mount
* [x] document consumer views (`docs/consumer-views.md`)

---

### Phase 4B Notes

* Presenter controls structure; encoder controls naming
* Flat view flattens directory structure but preserves encoder-derived filenames
* Non-game content is reduced to leaf filenames to ensure valid filesystem entries
* Filename collision handling is deferred to Phase 4C
* No changes were made to normalization or core pipeline stages

---

### Phase 4B Outcome

Retromount can now project the same normalized content into multiple filesystem layouts.

This establishes a clean separation between:

* data model (normalization)
* structure (presentation)
* naming (encoding)

and enables future extensibility (Phase 4D).

---

## Phase 4C — Presentation Policy

### Phase 4C Goal

Make output deterministic, predictable, and configurable.

---

### Phase 4C Branch

feature/phase4c-presentation-policy

---

### Phase 4C Success Criteria

* Stable naming rules
* Clear handling of duplicates and ambiguity
* Configurable behaviour (lightweight)

---

### Phase 4C Work Items

* Naming policy
* Conflict resolution rules
* Optional filtering based on existing data

---

## Phase 4D — Composition & Extensibility Foundations

### Phase 4D Goal

Make presenter and encoder composition explicit, configurable, and extensible.

Status: Implemented

---

### Phase 4D Branch

feature/phase4d-config-driven-composition

---

### Phase 4D Success Criteria

* [x] presenter selection is registry-backed
* [x] encoder selection is registry-backed
* [x] presenter/encoder composition is explicit
* [x] configuration can select presenter and encoder per view
* [x] defaults are applied when fields are omitted
* [x] legacy hardcoded presenter selection removed

---

### Phase 4D Outcome

Retromount now supports explicit, registry-backed composition of presenters and encoders.

Views can be configured independently, allowing the same underlying content
to be exposed in different layouts and representations without duplication.

This establishes the foundation for future extension work in Phase 5.

---

## Guiding Principle

Phase 3 proved the model.  
Phase 4 proves the model works in reality.

---

## Maintenance Rules

* This is a living document — update it if reality diverges
* If scope changes, document why
* Keep phases focused and bounded
