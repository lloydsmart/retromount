# Architecture Boundaries

This document describes the intended architectural boundaries of Retromount following Phase 3 consolidation (see D-001).

---

## Pipeline Stages

1. Input
2. Identify
3. Decode
4. Normalize (Core Model)
5. Present / Encode

---

## Input and Discovery Layers

Retromount distinguishes between two related but separate concepts within the "Input" stage.

### 1. Discovery Layer (Built-in Input Handlers)

Implemented in: `src/builtin_inputs` (formerly `src/inputs`)

This layer is responsible for:

- discovering and enumerating content from filesystem paths and archives
- expanding inputs (directories, ZIPs, etc.) into `VirtualFile` representations
- handling container-specific traversal (e.g. walking directories, reading archive contents)

Key components:

- `InputHandler`
- `DirectoryInputHandler`
- `FileInputHandler`
- `CueInputHandler`
- `ZipInputHandler`
- `InputRegistry`
- `engine::Loader`

This layer answers:

> “What files exist, and how do we enumerate them?”

---

### 2. Pipeline Input Layer (Semantic Ingestion)

Implemented in: `src/input`

This layer is responsible for:

- consuming source objects
- identifying content types
- decoding content into structured representations
- producing normalized `Content` / `GameContent` models

Key components:

- `InputSource`
- `InputIdentifier`
- `InputDecoder`
- `DirectoryInputSource`
- `ZipInputSource`
- `BasicInputIdentifier`
- `BasicInputDecoder`

This layer answers:

> “What is this content, and how should it be interpreted?”

---

### Relationship Between the Layers

These layers are adjacent but distinct:

- the **discovery layer** operates on *paths and containers*
- the **pipeline layer** operates on *content and semantics*

The discovery layer may feed into the pipeline layer, but they are not interchangeable and should not be conflated.

---

### Design Principles

- discovery concerns must not leak into semantic processing
- semantic processing must not assume filesystem-specific behaviour
- the boundary between these layers should remain explicit and stable

---

## Stage Contracts

### Input → Identify

- Input provides raw sources (files, archives, etc.)
- No semantic interpretation here
- May originate from either discovery layer or direct pipeline input sources

### Identify → Decode

- Identify determines what something *might be*
- Decode performs actual parsing/understanding

### Decode → Core Model

- Produces `Content` / `GameContent`
- Must not leak container-specific details

### Core Model → Presenter

- Presenter consumes normalized model
- Must not re-interpret raw inputs

---

## Known Boundary Violations (to fix)

- [ ] F-001: input vs inputs naming ambiguity (pending rename to `builtin_inputs`)
- [ ] F-002: loader vs pipeline orchestration ambiguity
- [ ] F-003: presenter vs encoder responsibility boundary
- [ ] F-004: potential core model presentation leakage
