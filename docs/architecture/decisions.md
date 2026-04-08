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
**Status:** Implemented  
**Related:** F-003  
**Issue:** [#43](https://github.com/lloydsmart/retromount/issues/43)

---

### Decision

Retromount will retain both Presenter and Encoder abstractions with a clearly defined responsibility boundary:

- **Presenter** is responsible for output structure, grouping, and layout
- **Encoder** is responsible for per-item materialization, naming, and representation

These roles are complementary and must remain distinct.

---

### Responsibility Breakdown

#### Presenter

The Presenter operates on normalized content (`NormalizedContent`, `GameContent`, `GamePart`) and is responsible for:

- defining the output structure (directories and hierarchy)
- grouping related content (e.g. multi-disc games)
- determining layout decisions (e.g. root vs nested placement)
- deciding when compound artifacts are required (e.g. playlists)
- constructing the logical VFS tree

The Presenter answers:

> “What should the output look like?”

The Presenter must **not**:

- implement filename or extension rules
- perform file materialization logic
- determine backing type (inline vs source-backed)
- reinterpret raw inputs or bypass normalized content

---

#### Encoder

The Encoder operates at the level of individual output items and is responsible for:

- generating filenames and extensions
- defining how a content item is materialized as a file
- determining file backing (inline vs source-backed)
- generating file contents (e.g. playlists)
- mapping content parts to file representations
- applying representation-specific transformations (e.g. `.nfo` → `.txt`)

The Encoder answers:

> “How is this specific item represented?”

The Encoder must **not**:

- define directory structure or layout
- group content into collections
- make global presentation decisions

---

### Examples

| Concern                                 | Owner     |
|-----------------------------------------|-----------|
| Multi-disc games as directories         | Presenter |
| Platform-based folder structure         | Presenter |
| Whether to generate a playlist          | Presenter |
| Disc filename `(Disc 1).cue`            | Encoder   |
| ROM filename and extension              | Encoder   |
| Text file normalization (`.nfo → .txt`) | Encoder   |
| Binary output naming (`.bin`)           | Encoder   |
| File backing (inline vs source-backed)  | Encoder   |
| Playlist file contents                  | Encoder   |

---

### Rationale

Separating structure from representation provides:

- clear and enforceable architectural boundaries
- elimination of duplicated representation logic
- easier reasoning about output behaviour
- improved extensibility for alternate output formats
- a clean extension point for future plugin systems

This aligns with the pipeline design:

- normalization defines *what the content is*
- presentation defines *how it is organized*
- encoding defines *how it is materialized*

---

### Consequences

- naming and representation logic has been removed from Presenter implementations
- file materialization (including backing type and generated content) is now owned by Encoder
- Presenter operates purely as a structural transformation over normalized content
- `OutputEncoder` becomes a primary extension point for output formats

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

This decision has been fully implemented.  
The Presenter/Encoder boundary is now enforced in code, with no remaining responsibility overlap.

---

## D-004: Ensure core model remains presentation-agnostic

**Date:** 2026-03-26  
**Status:** Implemented  
**Related:** F-004
**Issue:** [#44](https://github.com/lloydsmart/retromount/issues/44)

### D-004 Context

The core model should describe content semantically, independently from how that content is presented or materialized in output views.

Earlier iterations of the model included filename-oriented data in `RomPart`, which introduced representation concerns into the normalization layer.

This created tension with D-003, which established that naming and materialization belong to the Encoder, not the core model.

### D-004 Decision

The normalized core model (`NormalizedContent`, `GameContent`, `GamePart`) will contain only semantic information about content and must not encode presentation or representation decisions.

- The core model defines what the content is
- The Presenter defines how content is structured
- The Encoder defines how content is represented

### D-004 Rules

The core model may include:

- intrinsic identifiers (e.g. title, platform)
- structural relationships (e.g. multi-disc ordering)
- source references and sizes

The core model must not include:

- filenames or extensions
- formatting or naming conventions
- output-specific representation details

### D-004 Changes

- removed filename-related data from `RomPart`
- ensured all filename generation is handled exclusively by `OutputEncoder`
- updated encoder logic to derive filenames from source information

### D-004 Rationale

This enforces a strict separation between semantics and representation:

- eliminates presentation leakage into the core model
- ensures consistency with D-003 (Presenter vs Encoder boundary)
- enables alternate output formats without modifying core structures
- simplifies reasoning about data flow through the pipeline

### D-004 Consequences

- the core model is now fully presentation-agnostic
- all naming and representation logic is centralized in the Encoder
- Presenter operates purely on structure without relying on embedded naming hints
- future encoders can define entirely different naming schemes without impacting core logic

### D-004 Alternatives considered

- **Retain filename in `RomPart` for convenience**  
  Rejected — introduces representation concerns into the core model and duplicates encoder responsibility

- **Remove disc numbering from `DiscPart`**  
  Rejected — disc ordering is intrinsic to the content and required for correct grouping and semantics

### D-004 Notes

This decision builds directly on D-003 and completes the separation between:

- semantic model (core)
- structural layout (Presenter)
- representation (Encoder)

The pipeline is now cleanly layered end-to-end.

---

## D-005: Separate decoded content from normalized content

**Date:** 2026-03-26  
**Status:** Implemented  
**Related:** F-005  
**Issue:** [#53](https://github.com/lloydsmart/retromount/issues/53)

### D-005 Context

Retromount previously used a single `Content` model to represent both decoded input artifacts and normalized semantic output.

This meant the same model family spanned multiple pipeline stages:

- decode produced `Content`
- normalize consumed `Content` and produced `Content`
- present consumed normalized `Content`
- encode consumed normalized `Content`

In practice, later stages did not accept all `Content` variants equally.

Presenter and encoder logic had to assume that certain pre-normalized variants such as `Rom` and `Disc` should no longer appear after normalization, which made the type boundary broader than the real stage contract.

### D-005 Decision

Retromount now uses an explicit type separation between decoded content and normalized content.

- **Decoded content** represents the direct output of the decode stage
- **Normalized content** represents the semantic output of the normalize stage
- presentation and encoding stages consume only normalized content
- pre-normalized variants are no longer representable at post-normalization stage boundaries

This makes the decode → normalize → present pipeline contract explicit in the type model.

### D-005 Implemented Model Shape

The model is now split into two stage-aligned layers:

- a decoded content model for artifacts discovered and interpreted from input sources
- a normalized content model for semantic entities that are valid after normalization

In practice:

- ROM and disc artifacts belong to the decoded model
- game-level semantic entities belong to the normalized model
- presenter and encoder interfaces consume normalized content only

### D-005 Rationale

This change strengthens the pipeline architecture by making stage validity explicit rather than conventional.

It provides:

- clearer stage contracts
- elimination of impossible states from later pipeline stages
- removal of `unreachable!()` branches caused by over-broad content types
- stronger guarantees for future plugin/extensibility work
- easier reasoning about where transformations occur in the pipeline

This aligns with the architectural direction established by earlier review work:

- D-002 clarified orchestration boundaries
- D-003 clarified presenter vs encoder responsibilities
- D-004 clarified semantic vs presentation boundaries
- D-005 clarified decode vs normalize boundaries

### D-005 Consequences

- decode no longer produces the same top-level model used after normalization
- normalization is now an explicit transformation between two model families
- presenter and encoder APIs consume normalized content only
- the legacy `Content` abstraction has been removed
- plugin/extensibility boundaries are clearer and safer

### D-005 Alternatives considered

- **Keep a single `Content` model and document stage contracts more clearly**  
  Rejected — leaves invalid post-normalization states representable and preserves over-broad interfaces

- **Introduce only a wrapper/newtype around `Content` after normalization**  
  Rejected — improves the boundary somewhat, but still retains a weaker type distinction than the pipeline now warrants

- **Narrow presenter/encoder interfaces without splitting the model family**  
  Rejected — reduces some ambiguity, but does not fully clarify the decode → normalize transition

### D-005 Notes

This decision has been fully implemented.

The pipeline is now explicitly layered as:

- decoded content
- normalized content
- presented/encoded output

See `boundaries.md` for the detailed model shape and pipeline contracts.

---

## D-006: Resolve presenters via registry-backed composition

**Date:** 2026-04-08  
**Status:** Implemented  
**Related:** F-104  
**Issue:** [#61](https://github.com/lloydsmart/retromount/issues/61)

### D-006 Context

Retromount introduced multiple presenter implementations in Phase 4B, but presenter selection remained hardcoded in runtime composition.

This meant presenter extensibility existed at the type level, but not yet at the composition level.

### D-006 Decision

Retromount will resolve presenters through a registry-backed composition model rather than enum-based or hardcoded branching.

Presenter selection becomes a first-class composition concern.

### D-006 Consequences

- presenter selection now flows through `PresenterRegistry`
- built-in presenter validation and lookup are registry-backed
- `PresenterKind` has been removed
- configured and runtime execution resolve presenters by name
- configuration can select presenters per view

### D-006 Rationale

This change:

- removes hardcoded presenter branching from runtime composition
- makes presenter selection consistent with extensibility goals
- provides a cleaner boundary for future plugin-oriented presenter integrations
- aligns the runtime model with the architecture established in Phase 4B

### D-006 Alternatives considered

- **Keep presenter branching in composition code**  
  Rejected — preserves unnecessary coupling and weakens extensibility

- **Defer all presenter selection cleanup to a future plugin phase**  
  Rejected — leaves the built-in composition model inconsistent and incomplete

### D-006 Notes

This decision has been fully implemented.

Presenter selection is now registry-backed and configuration-driven.

---

## D-007: Make presenter and encoder composition explicit

**Date:** 2026-04-08  
**Status:** Implemented  
**Related:** F-106  
**Issue:** [#63](https://github.com/lloydsmart/retromount/issues/63)

### D-007 Context

Retromount already separated structure (Presenter) from representation (Encoder), but the actual pairing of presenters and encoders remained implicit.

Encoders were effectively selected inside presenter construction, which prevented composition from being treated as an explicit runtime concern.

### D-007 Decision

Retromount will compose presenters and encoders explicitly through the composition layer.

Presenter and encoder selection are both first-class runtime/configuration concerns.

### D-007 Consequences

- encoder selection now flows through `EncoderRegistry`
- presenters depend on `OutputEncoder` rather than concrete encoder ownership
- the composition layer explicitly selects both presenter and encoder
- configured views can specify presenter and encoder independently
- default built-in composition remains `grouped` + `basic` when omitted

### D-007 Rationale

This change:

- completes the Presenter/Encoder separation established by D-003
- removes hidden encoder coupling from presenter construction
- makes composition easier to reason about and configure
- provides the foundation for future extension/plugin work without contaminating core behaviour

### D-007 Alternatives considered

- **Keep encoder selection implicit inside presenter construction**  
  Rejected — leaves composition hidden and weakens extensibility

- **Expose only presenter selection and defer encoder composition**  
  Rejected — produces an incomplete composition model

### D-007 Notes

This decision has been fully implemented.

Retromount now supports explicit, registry-backed presenter and encoder composition, including per-view configuration.
