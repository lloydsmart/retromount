# Architecture Boundaries

This document describes the intended architectural boundaries of Retromount following Phase 3 consolidation and D-002 (pipeline orchestration unification).

---

## Canonical Orchestration Model

Retromount uses a single orchestration model based on the Phase 3 pipeline:

1. Input
2. Identify
3. Decode
4. Normalize (Core Model)
5. Present / Encode (VFS)

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

### 5. Present / Encode

Responsible for:

- transforming normalized content into a view
- producing filesystem-like output via the VFS

Key components:

- `OutputPresenter`
- `GenericPresenter`
- `BasicEncoder`
- `VfsDirectory`
- `VfsFile`

This stage answers:

> “How should this content be represented to a consumer?”

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
- Must not leak container or source-specific details
- Must not encode presentation decisions

---

### Core Model → Presenter

- Presenter consumes normalized model
- Must not re-interpret raw inputs
- Must not depend on source/container details

---

## Design Principles

- **Single orchestration model**  
  All processing flows through the pipeline

- **Separation of concerns**
  - Input handles enumeration
  - Identify/Decode handle interpretation
  - Core model represents semantics
  - Presenter/Encoder handle output

- **Presentation independence**
  The core model must not encode output-specific assumptions

- **Extensibility**
  Each stage should be replaceable or extensible without affecting others

---

## Known Boundary Concerns

- [x] F-001: input vs inputs naming ambiguity (resolved via `builtin_inputs` rename)
- [x] F-002: loader vs pipeline orchestration ambiguity (resolved via D-002)
- [ ] F-003: presenter vs encoder responsibility boundary
- [ ] F-004: potential core model presentation leakage
