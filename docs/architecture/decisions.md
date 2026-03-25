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
