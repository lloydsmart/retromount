# Architecture Boundaries

This document describes the intended architectural boundaries of Retromount following Phase 3 consolidation and D-002/D-003.

---

## Canonical Orchestration Model

Retromount uses a single orchestration model based on the Phase 3 pipeline:

1. Input
2. Identify
3. Decode
4. Normalize (Core Model)
5. Present (structure)
6. Encode (materialization → VFS)

All execution paths (preview, inspect, and configured/runtime execution) flow through this pipeline.

There is no alternative orchestration path.

---

## Pipeline Stages

### 1. Input

Implemented in: `src/input`

Responsible for:

- enumerating content from sources (directories, archives, files)
- abstracting over container formats
- producing a stream of input items for processing

Key components:

- `InputSource`
- `DirectoryInputSource`
- `ZipInputSource`

This stage answers:

> “What content is available for processing?”

---

### 2. Identify

Responsible for:

- determining what each input item might represent
- selecting an appropriate decoding strategy

Key components:

- `InputIdentifier`
- `BasicInputIdentifier`

This stage answers:

> “What is this likely to be?”

---

### 3. Decode

Responsible for:

- parsing input into structured representations
- interpreting formats (ROMs, CUE/BIN, etc.)

Key components:

- `InputDecoder`
- `BasicInputDecoder`

This stage answers:

> “What is this, structurally?”

---

### 4. Normalize (Core Model)

Implemented in: `src/core`

Responsible for:

- producing normalized, presentation-agnostic representations of content
- modelling games, discs, and other semantic entities

Key types:

- `Content`
- `GameContent`
- `GamePart`
- `Disc`

This stage answers:

> “What is this content, independent of how it will be shown?”

---

### 5. Present (structure)

Responsible for:

- defining output structure (directories and hierarchy)
- grouping related content (e.g. multi-disc games)
- determining layout decisions (e.g. root vs nested placement)
- deciding when compound artifacts are required (e.g. playlists)
- constructing the logical VFS tree

Key components:

- `OutputPresenter`
- `GenericPresenter`

This stage answers:

> “What should the output look like?”

The Presenter must **not**:

- generate filenames or extensions
- determine file backing (inline vs source-backed)
- generate file contents
- perform representation-specific transformations

---

### 6. Encode (materialization → VFS)

Responsible for:

- generating filenames and extensions
- mapping content to file representations
- determining file backing (inline vs source-backed)
- generating file contents (e.g. playlists)
- applying representation-specific transformations

Key components:

- `OutputEncoder`
- `BasicEncoder`
- `VfsFile`

This stage answers:

> “How is this specific item represented?”

The Encoder must **not**:

- define directory structure or layout
- group content into collections
- make global presentation decisions

---

## Stage Contracts

### Input → Identify

- Input produces raw content items
- No semantic interpretation occurs here
- Must not depend on content type assumptions

---

### Identify → Decode

- Identify suggests possible content type(s)
- Decode performs actual parsing and validation

---

### Decode → Core Model

- Produces `Content` / `GameContent`
- May retain source provenance needed for semantic interpretation
- Must not encode presentation decisions such as filenames, extensions, or naming conventions

---

### Core Model → Presenter

- Presenter consumes normalized model
- Must not re-interpret raw inputs
- Must not depend on source/container details

---

### Presenter → Encoder

- Presenter defines structure and grouping
- Encoder defines representation of each item
- Presenter must not implement representation logic
- Encoder must not influence structure

---

## Design Principles

- **Single orchestration model**  
  All processing flows through the pipeline

- **Strict separation of concerns**
  - Input handles enumeration
  - Identify/Decode handle interpretation
  - Core model represents semantics
  - Presenter defines structure
  - Encoder defines representation

- **Presentation independence**
  The core model must not encode output-specific assumptions

- **Extensibility**
  Each stage should be replaceable or extensible without affecting others

---

## Known Boundary Concerns

- [x] F-001: input vs inputs naming ambiguity (resolved via `builtin_inputs` rename)
- [x] F-002: loader vs pipeline orchestration ambiguity (resolved via D-002)
- [x] F-003: presenter vs encoder responsibility boundary (resolved via D-003)
- [x] F-004: core model presentation leakage (resolved via D-004)
