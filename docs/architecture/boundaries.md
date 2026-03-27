# Architecture Boundaries

This document describes the intended architectural boundaries of Retromount following Phase 3 consolidation and D-002 through D-005.

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

### 4. Normalize (Decoded → Normalized Boundary)

Implemented in: `src/core`

Responsible for:

- producing normalized, presentation-agnostic representations of content
- modelling games, discs, and other semantic entities

Key types:

- `DecodedContent`
- `NormalizedContent`
- `GameContent`
- `GamePart`

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

In the default Phase 3 implementation, the presenter composes an encoder while constructing the VFS tree.

Conceptually:

- presentation owns structure and layout decisions
- encoding owns file materialization

This preserves the architectural boundary while allowing a simple default composition.

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

### Decode → Normalize

- Decode produces `DecodedContent`
- Normalize consumes `DecodedContent` and produces `NormalizedContent`
- Source provenance may be retained where needed for semantic interpretation
- Normalized content must not encode presentation decisions such as filenames, extensions, or naming conventions
- Normalization is the only stage where decoded artifacts may be aggregated, merged, or transformed into higher-level semantic entities (e.g. discs → games)

---

### Normalized Model → Presenter

- Presenter consumes `NormalizedContent`
- Must not re-interpret raw inputs
- Must not depend on decoder-specific details

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
- [x] F-005: decoded vs normalized content boundary defined (D-005 implemented)

## Decode → Normalize Boundary

Following D-005, Retromount separates decoded content from normalized content.

### Decoded content

The decode stage produces `DecodedContent`, representing raw interpreted artifacts from input sources.

```rust
pub enum DecodedContent {
    Bytes(BytesContent),
    Rom(DecodedRomContent),
    Disc(DecodedDiscContent),
    Text(TextContent),
}
```

### Normalized content

The normalize stage produces `NormalizedContent`, representing semantic entities valid for presentation.

```rust
pub enum NormalizedContent {
    Bytes(BytesContent),
    Game(GameContent),
    Text(TextContent),
}
```

### Pipeline contracts

- decoder → `Vec<DecodedContent>`
- normalizer → `Vec<NormalizedContent>`
- presenter → consumes `NormalizedContent`
- encoder → consumes `NormalizedContent`

Pre-normalized artifacts such as ROMs and discs are not visible beyond the normalization stage.

### Rationale

This boundary ensures:

- explicit stage contracts
- elimination of impossible post-normalization states
- clearer plugin and extension boundaries
