# Review Findings

## Status legend

- Open
- Accepted
- Rejected
- Resolved

---

## F-001: `src/input` and `src/inputs` are confusingly named

**Status:** Open  
**Issue:** [#39](https://github.com/lloydsmart/retromount/issues/39)

### Summary

The codebase contains both `src/input` and `src/inputs`, which represent different concepts but are named too similarly.

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

- `src/inputs` contains the built-in path discovery handlers:
  - `DirectoryInputHandler`
  - `FileInputHandler`
  - `CueInputHandler`
  - `ZipInputHandler`
  - `register_builtin_inputs()`

- `src/inputs` is used by:
  - `core::InputRegistry`
  - `engine::Loader`

- The two modules serve different roles, but their names are too similar and make the architecture harder to understand at a glance.

### Why it matters

This makes the intended architecture harder to understand and may conceal overlapping responsibilities.

### Options

- Rename one or both modules
- Consolidate into a single model
- Keep separate but document more clearly

### Decision

D-001
---

## F-002: `engine::Loader` and the Phase 3 pipeline create ambiguous system entry points

**Status:** Open

### Summary

The codebase appears to expose two different ingestion/orchestration paths:

- a loader-oriented path centered around `engine::Loader`
- a Phase 3 pipeline-oriented path centered around the newer input/identify/decode/present flow

It is not yet clear which of these is intended to be the canonical long-term entry point.

### Evidence

- `engine::Loader` exists as a higher-level mechanism for loading and processing inputs
- the Phase 3 work introduced a pipeline-oriented flow based on input source, identification, decoding, normalization, and presentation
- both paths appear to participate in turning source material into virtualized output structures

### Why it matters

If the project has multiple orchestration paths without a clearly defined long-term role for each, it becomes harder to:

- understand the intended architecture
- add new features without duplicating integration work
- define stable extension points for future plugins or built-in modules
- know where cleanup and consolidation should happen

This ambiguity also increases the risk of architectural drift, where new work lands in whichever path is most convenient rather than whichever is intended.

### Options

- Declare one path as the canonical long-term entry point and reduce the other to an internal helper or transitional layer
- Keep both paths, but document them as distinct layers with clearly separated responsibilities
- Consolidate both paths into a single orchestration model

### Decision

TBD
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
