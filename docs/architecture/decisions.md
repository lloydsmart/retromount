# Architecture Decisions

This document records significant architectural decisions made during the development of RetroMount.

---

## Status legend

- Proposed
- Accepted
- Implemented
- Superseded
- Rejected

---

## D-001: Separate pipeline input layer from built-in discovery handlers

**Date:** 2026-03-25  
**Status:** Accepted  
**Related:** F-001  
**Issue:** [#39](https://github.com/lloydsmart/retromount/issues/39)

### D-001 Context

The codebase contains two similarly named modules:

- `src/input`
- `src/inputs`

Although they appear similar, they serve different architectural purposes:

- The pipeline input layer (`src/input`) operates on source objects and drives semantic processing (identify → decode → normalize → present)
- The discovery handler layer (`src/inputs`) operates on filesystem/archive paths and expands them into `VirtualFile` structures via `InputRegistry` and `Loader`

These are adjacent but distinct concerns:

- discovery: "what files exist and how do we enumerate them?"
- ingestion: "what is this content and how do we interpret it?"

### D-001 Decision

Retromount will retain both the Phase 3 pipeline input layer and the discovery handler layer as separate concepts.

The modules will be clarified as follows:

- `src/input` will remain the pipeline ingestion layer (InputSource, Identifier, Decoder)
- `src/inputs` will be renamed to `src/builtin_inputs` to reflect its role as the built-in discovery handler set

### D-001 Consequences

- `src/inputs` will be renamed to `src/builtin_inputs`
- all references to `crate::inputs` will be updated accordingly
- module-level documentation will be updated to clarify responsibilities
- `src/input` remains unchanged
- broader orchestration questions (Loader vs pipeline) will be addressed under F-002

### D-001 Alternatives considered

- **Consolidate both modules into a single ingestion model**  
  Rejected for now — would introduce unnecessary risk without first resolving higher-level orchestration questions (F-002)

- **Rename both modules simultaneously**  
  Deferred — renaming `src/inputs` alone provides sufficient clarity with less churn

- **Remove loader/discovery layer entirely**  
  Rejected — still actively used and represents a valid lower-level abstraction

### D-001 Notes

This decision focuses on clarity and boundary definition only. No intended behaviour changes are introduced.

---

## D-002: Consolidate Retromount onto a single orchestration model

**Date:** 2026-03-25  
**Status:** Implemented  
**Related:** F-002  
**Issue:** [#42](https://github.com/lloydsmart/retromount/issues/42)

### D-002 Context

The system previously exposed two orchestration paths:

1. A loader-oriented path centered around `engine::Loader`
2. A Phase 3 pipeline-oriented path (input → identify → decode → normalize → present)

Maintaining both created architectural ambiguity and duplicated responsibility.

### D-002 Decision

Retromount will converge on a single orchestration model built around the Phase 3 pipeline.

Configured and runtime execution are migrated onto the pipeline, and `engine::Loader` is removed as a top-level orchestration mechanism.

### D-002 Consequences

- The loader-based orchestration path has been removed
- All configured/runtime execution now flows through the Phase 3 pipeline
- Discovery responsibilities have been re-homed into pipeline-compatible supporting layers
- The pipeline becomes the canonical data flow for the system

### D-002 Notes

This decision has been implemented. The Loader and associated discovery orchestration model have been removed.  

---

## D-003: Define clear boundary between Presenter and Encoder responsibilities

**Date:** 2026-03-25  
**Related findings:** F-003  
**Issue:** #43  
**Status:** Accepted

---

### Decision

Retromount will retain both Presenter and Encoder abstractions with a clearly defined responsibility boundary:

- **Presenter** is responsible for output structure, grouping, and layout
- **Encoder** is responsible for per-item materialization, naming, and representation

These roles are complementary and must remain distinct.

---

### Responsibility Breakdown

#### Presenter

The Presenter operates on normalized content (`Content`, `GameContent`, `GamePart`) and is responsible for:

- defining the output structure (directories and hierarchy)
- grouping related content (e.g. multi-disc games)
- determining layout decisions (e.g. root vs nested placement)
- deciding when compound artifacts are required (e.g. M3U playlists)
- constructing the logical VFS tree

The Presenter answers:

> “What should the output look like?”

The Presenter must **not**:

- implement filename or extension rules
- perform low-level file representation logic
- reinterpret raw inputs or bypass normalized content

---

#### Encoder

The Encoder operates at the level of individual output items and is responsible for:

- generating filenames and extensions
- defining how a content item is materialized as a file
- mapping content parts to VFS file nodes
- determining whether content is inline, generated, or source-backed
- applying representation-specific transformations (e.g. `.nfo` → `.txt`)

The Encoder answers:

> “How is this specific item represented?”

The Encoder must **not**:

- define directory structure or layout
- group content into collections
- make global presentation decisions

---

### Examples

| Concern                              | Owner     |
|--------------------------------------|-----------|
| Multi-disc games as directories      | Presenter |
| Platform-based folder structure      | Presenter |
| Whether to generate an M3U playlist  | Presenter |
| Disc filename `(Disc 1).cue`         | Encoder   |
| ROM filename and extension           | Encoder   |
| Text file normalization (`.nfo → .txt`) | Encoder |
| Binary output naming (`.bin`)        | Encoder   |

---

### Rationale

Separating structure from representation provides:

- clearer architectural boundaries
- easier reasoning about output behavior
- improved extensibility for future output formats
- better support for plugin-style view customization

This aligns with the overall pipeline design:

- normalization defines *what the content is*
- presentation defines *how it is organized*
- encoding defines *how it is materialized*

---

### Consequences

- existing Presenter and Encoder implementations should be reviewed for responsibility leaks
- logic currently implemented in the wrong layer should be migrated:
  - structural/grouping logic → Presenter
  - naming/representation logic → Encoder
- future features (e.g. alternate views, export formats) should target the appropriate abstraction

---

### Alternatives considered

- **Collapse Presenter and Encoder into a single abstraction**  
  Rejected — would blur responsibilities and reduce clarity and extensibility

- **Move all logic into Presenter**  
  Rejected — would overload Presenter with representation concerns

- **Move all logic into Encoder**  
  Rejected — would force Encoder to manage structure and grouping, breaking separation of concerns

---

### Notes

This decision clarifies the output boundary without introducing new functionality.  
Implementation will focus on aligning existing code with this responsibility split.
