# Phase 4B — Consumer Views

This document defines the concrete implementation plan and scope for Phase 4B.

Phase 4B builds on the Phase 4A mountable filesystem and introduces support for multiple presentation layouts over the same normalized content.

---

## Goal

Support multiple filesystem layouts for different consumers without changing the core pipeline.

Specifically:

> The same normalized content can be presented as different Virtual File System (VFS) trees via interchangeable presenter implementations.

---

## Scope

Phase 4B introduces:

- explicit presenter selection in runtime/CLI
- multiple presenter implementations
- the ability to render the same input into different filesystem layouts

---

## Non-Goals

Phase 4B explicitly does **not** include:

- naming policy redesign (Phase 4C)
- duplicate/conflict resolution strategy changes (Phase 4C)
- metadata enrichment or scraping
- performance optimisation or caching
- plugin loading or external extension (Phase 4D)

---

## Success Criteria

- at least two presenter implementations exist
- the same normalized input produces different VFS layouts
- presenter selection is possible via CLI
- both `inspect` and `mount` support presenter selection
- core pipeline stages (Input → Normalize) remain unchanged
- no presentation logic leaks into normalization
- encoder remains reusable across presenters

---

## Initial Presenter Set

### Grouped Presenter (default)

Represents the current layout behaviour.

Characteristics:

- groups content by title (and/or platform if already implemented)
- multi-disc titles are grouped together
- playlists (e.g. `.m3u`) appear alongside grouped content

Example:

```text
PlayStation/
  Final Fantasy VII/
    Final Fantasy VII.m3u
    Final Fantasy VII (Disc 1).cue
    Final Fantasy VII (Disc 2).cue
```

---

### Flat Presenter

Provides a minimal, non-grouped layout.

Characteristics:

- no directory grouping by title
- all items appear in a single directory (or minimal hierarchy)
- filenames remain derived via encoder rules
- multi-disc relationships are not expressed via directories

Example:

```text
Final Fantasy VII (Disc 1).cue
Final Fantasy VII (Disc 2).cue
Final Fantasy VII.m3u
Metal Gear Solid (Disc 1).cue
Metal Gear Solid (Disc 2).cue
```

---

## Architecture

### Presenter Role

The presenter is responsible for:

- directory structure
- grouping
- hierarchy
- deciding which logical items appear in which directories

The presenter is **not** responsible for:

- filename generation
- file extensions
- file content generation (e.g. `.m3u`)
- encoding or materialization

---

### Encoder Role

The encoder remains responsible for:

- filename derivation
- file extensions
- inline file generation (e.g. playlists)
- mapping logical items to file backing (source vs inline)

---

### Pipeline Integrity

The canonical pipeline remains unchanged:

1. Input  
2. Identify  
3. Decode  
4. Normalize  
5. Present (multiple implementations)  
6. Encode  
7. VFS  

No additional orchestration paths are introduced.

---

## CLI Interface

Presenter selection is exposed via a simple flag:

```bash
retromount inspect <input> --view grouped
retromount inspect <input> --view flat

retromount mount <input> <mountpoint> --view grouped
retromount mount <input> <mountpoint> --view flat
```

Default:

```text
--view grouped
```

---

## Implementation Plan

### Slice 1 — Presenter Selection

- introduce presenter selection in CLI/runtime wiring
- default to current behaviour

---

### Slice 2 — Formalize Current Presenter

- explicitly define current presenter as `grouped`
- ensure behaviour remains unchanged

---

### Slice 3 — Flat Presenter

- implement alternative layout with minimal grouping
- reuse encoder logic where possible

---

### Slice 4 — CLI Integration

- add `--view` flag to `inspect` and `mount`
- wire presenter selection through pipeline execution

---

### Slice 5 — Tests

- verify same input produces different VFS trees
- cover multi-disc scenarios
- ensure playlists are still correctly generated
- ensure encoder behaviour remains consistent

---

### Slice 6 — Documentation

- update roadmap status as Phase 4B progresses
- document available views and behaviour

---

## Guardrails

To prevent scope creep into later phases:

- do not introduce naming policy configuration (Phase 4C)
- do not implement conflict resolution strategies beyond current behaviour
- do not introduce dynamic or user-defined layouts
- do not modify normalized data structures for presentation purposes
- do not introduce caching or performance optimisations

---

## Definition of Done

Phase 4B is complete when:

- at least two distinct presenter implementations exist
- CLI supports selecting between them
- the same input produces different filesystem layouts
- behaviour is stable and test-covered
- architecture boundaries remain intact

---

## Notes

Phase 4B proves that:

> Retromount can project the same underlying content into multiple filesystem representations without changing its core model.

This is a key step toward plugin extensibility in Phase 4D.
