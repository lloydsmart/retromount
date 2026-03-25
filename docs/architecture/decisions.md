# Architecture Decisions

## D-001: Separate pipeline input layer from built-in discovery handlers

**Date:** 2026-03-25  
**Related findings:** F-001  
**Issue:** #39

### Decision

Retromount will retain both the Phase 3 pipeline input layer and the loader/discovery handler layer as separate concepts.

The modules will be clarified as follows:

- `src/input` will remain the pipeline ingestion layer (InputSource, Identifier, Decoder)
- `src/inputs` will be renamed to `src/builtin_inputs` to reflect its role as the built-in discovery handler set

### Rationale

Although `src/input` and `src/inputs` appear similar by name, they serve different architectural purposes:

- The pipeline input layer (`src/input`) operates on source objects and drives semantic processing (identify → decode → normalize → present)
- The discovery handler layer (`src/inputs`) operates on filesystem/archive paths and expands them into `VirtualFile` structures via `InputRegistry` and `Loader`

These are adjacent but distinct concerns:

- discovery: "what files exist and how do we enumerate them?"
- ingestion: "what is this content and how do we interpret it?"

Keeping both layers allows the system to:

- support both loader-based and pipeline-based workflows
- maintain separation between low-level file discovery and higher-level content semantics
- evolve each layer independently

### Consequences

- `src/inputs` will be renamed to `src/builtin_inputs`
- all references to `crate::inputs` will be updated accordingly
- module-level documentation will be updated to clarify responsibilities
- `src/input` remains unchanged for now
- broader orchestration questions (Loader vs pipeline) will be addressed under F-002

### Alternatives considered

- **Consolidate both modules into a single ingestion model**  
  Rejected for now — would introduce unnecessary risk without first resolving higher-level orchestration questions (F-002)

- **Rename both modules simultaneously**  
  Deferred — renaming `src/inputs` alone provides sufficient clarity with less churn

- **Remove loader/discovery layer entirely**  
  Rejected — still actively used and represents a valid lower-level abstraction

### Notes

This decision focuses on clarity and boundary definition only. No intended behaviour changes are introduced
---
