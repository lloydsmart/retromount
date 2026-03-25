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

- preserve a clear distinction between discovery-oriented and pipeline-oriented responsibilities while broader orchestration questions are resolved
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

## D-002: Consolidate Retromount onto a single orchestration model

**Date:** 2026-03-25  
**Related findings:** F-002  
**Issue:** #42  
**Status:** Accepted in principle  

### Decision

Retromount will converge on a single orchestration model built around the Phase 3 pipeline.

Configured and runtime execution should be migrated onto the pipeline so that `engine::Loader` and the dual top-level orchestration model can be removed.

### Rationale

The pipeline provides the clearer long-term architecture:

- explicit processing stages
- normalized content flow
- presentation-aware output generation
- a better foundation for future extensibility

Maintaining both `Loader` and the pipeline as top-level orchestration paths introduces architectural ambiguity and unnecessary duplication.

### Consequences

- a dedicated follow-on branch will implement the migration
- `Loader` should be treated as a removal target rather than a permanent abstraction
- surviving discovery responsibilities must be re-homed into pipeline-compatible supporting layers

### Notes

This decision is accepted in principle on the architecture review branch. Implementation will be carried out separately and may refine the exact migration details.
