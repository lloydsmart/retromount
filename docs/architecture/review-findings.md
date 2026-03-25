# Review Findings

## Status legend

- Open
- Accepted
- Rejected
- Resolved

---

## F-001: `src/input` and `src/inputs` are confusingly named

**Status:** Resolved  
**Issue:** [#39](https://github.com/lloydsmart/retromount/issues/39)  
**Resolved by:** `935811d`

### Summary

The codebase contained both `src/input` and `src/inputs`, which represented different concepts but were named too similarly.

### Evidence

- `src/input` contains the Phase 3 pipeline-oriented ingestion abstractions and implementations:
  - `InputSource`
  - `InputIdentifier`
  - `InputDecoder`
  - `DirectoryInputSource`
  - `ZipInputSource`
  - `BasicInputIdentifier`
  - `BasicInputDecoder`

- `src/input` is used by:
  - `engine::pipeline`
  - `engine::preview`
  - `engine::inspect`

- `src/inputs` contained the built-in path discovery handlers:
  - `DirectoryInputHandler`
  - `FileInputHandler`
  - `CueInputHandler`
  - `ZipInputHandler`
  - `register_builtin_inputs()`

- `src/inputs` was used by:
  - `core::InputRegistry`
  - `engine::Loader`

- The two modules served different roles, but their names were too similar and made the architecture harder to understand at a glance.

### Why it matters

This made the intended architecture harder to understand and concealed the distinction between discovery-oriented and pipeline-oriented responsibilities.

### Options

- Rename one or both modules
- Consolidate into a single model
- Keep separate but document more clearly

### Decision

Resolved by D-001. `src/inputs` was renamed to `src/builtin_inputs`, clarifying its role as the built-in discovery handler set while retaining `src/input` as the pipeline ingestion layer  
---

## F-002: `engine::Loader` and the Phase 3 pipeline create ambiguous system entry points

**Status:** Resolved  
**Issue:** [#42](https://github.com/lloydsmart/retromount/issues/42)

### Summary

The codebase currently exposes two orchestration paths:

- a loader-oriented path centered around `engine::Loader`
- a Phase 3 pipeline-oriented path centered around the newer input/identify/decode/present flow

This duality should be removed so that Retromount has a single canonical orchestration model.

### Evidence

- `main.rs` uses `engine::loader::Loader` in configured runtime execution
- `Loader` orchestrates discovery through `InputRegistry` and returns either:
  - `Vec<VirtualFile>` via `discover_path()`, or
  - `GameImage` via `load_game_image()`
- `engine::pipeline` orchestrates the Phase 3 semantic flow:
  - `InputSource`
  - `InputIdentifier`
  - `InputDecoder`
  - `normalize_content()`
  - `OutputPresenter`
  - `VfsDirectory`
- `engine::preview` and `engine::inspect` use the pipeline path rather than `Loader`
- the two paths operate over different intermediate models (`VirtualFile` / `GameImage` vs `Content` / `VfsDirectory`)

### Why it matters

Retaining two top-level orchestration models makes the system harder to understand, harder to extend, and harder to clean up safely.

It also creates uncertainty about where new behavior should be added and weakens the architectural boundaries established in Phase 3.

### Options

- Keep both paths and document them as distinct long-term layers
- Treat the pipeline as canonical and `Loader` as transitional
- Migrate configured/runtime execution onto the Phase 3 pipeline and remove `Loader`

### Decision

Resolved by D-002. The loader-based orchestration path has been removed, and Retromount now uses the Phase 3 pipeline as its sole orchestration model.

Implementation proceeded on a dedicated follow-on branch: `feature/pipeline-orchestration-consolidation`.  
---

## F-003: Presenter and encoder responsibilities are not yet sharply defined

**Status:** Open

### Summary

The current output side of the architecture includes both presenter and encoder concepts, but the boundary between them is not yet fully clear.

In particular, naming, output structure, and representation logic may currently be split across these layers in ways that are harder to reason about than necessary.

### Evidence

- the project contains a presenter abstraction and a concrete `GenericPresenter`
- the project contains an encoder abstraction and a concrete `BasicEncoder`
- output concerns such as file naming, output layout, and content representation appear to involve both layers
- future Phase 4 work is expected to expand output/view behavior, increasing the importance of a clean separation here

### Why it matters

If presenter and encoder responsibilities are blurry, then future view work will be harder to implement cleanly.

This affects:

- where output structure should be decided
- where file naming should be decided
- where content representation/translation should be decided
- whether future output customizations can be added without modifying core logic

A weak boundary here also makes plugin-style extensibility more difficult, because it becomes unclear which interface a new output behavior should target.

### Options

- Define presenters as responsible for structure and encoders as responsible for representation/naming details
- Define presenters as pure view builders and move most filename/output translation logic into encoders
- Collapse the distinction if both abstractions are not providing enough independent value

### Decision

TBD
---

## F-004: The core model boundary needs review to ensure `Content` and `GameContent` remain presentation-agnostic

**Status:** Open

### Summary

The current core model appears to carry the project's normalized representation of games and other content, but it is not yet fully confirmed that these types are free from presentation-specific assumptions.

This finding is concerned with keeping the normalized model clean, reusable, and independent from output/view decisions.

### Evidence

- Phase 3 introduced stronger normalization around `Content` and `GameContent`
- the system now distinguishes between internal normalized content and output presentation
- multi-disc grouping, naming conventions, and playable output representation all put pressure on the core/output boundary
- some output behavior may currently depend on assumptions embedded in normalized model structures

### Why it matters

The core model should represent what the content *is*, not how it should be shown to a specific consumer.

If presentation-specific assumptions leak into `Content` or `GameContent`, then:

- alternative views become harder to implement
- internal semantics become tied to current output choices
- future plugin boundaries become less stable
- refactoring output behavior becomes more invasive than necessary

### Options

- Confirm the current model is sufficiently presentation-agnostic and document the boundary clearly
- Move presentation-oriented assumptions out of the core model and into presenter/encoder layers
- Introduce clearer intermediate types if the current model is carrying too many responsibilities

### Decision

TBD
