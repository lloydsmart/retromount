# Review Findings

This document captures architectural issues identified during review, along with their resolution status.

---

## Status legend

- Open
- Accepted
- Rejected
- Resolved

---

## F-001: `src/input` and `src/inputs` are confusingly named

**Status:** Resolved  
**Issue:** [#39](https://github.com/lloydsmart/retromount/issues/39)  
**Resolved by:** D-001 (`935811d`)

### F-001 Context

The codebase contained both `src/input` and `src/inputs`, which represented different concepts but were named too similarly.

### F-001 Evidence

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

### F-001 Why it matters

This made the intended architecture harder to understand and concealed the distinction between discovery-oriented and pipeline-oriented responsibilities.

### F-001 Options

- Rename one or both modules
- Consolidate into a single model
- Keep separate but document more clearly

### F-001 Decision

Resolved by D-001. `src/inputs` was renamed to `src/builtin_inputs`, clarifying its role as the built-in discovery handler set while retaining `src/input` as the pipeline ingestion layer.

---

## F-002: `engine::Loader` and the Phase 3 pipeline create ambiguous system entry points

**Status:** Resolved  
**Issue:** [#42](https://github.com/lloydsmart/retromount/issues/42)  
**Resolved by:** D-002

### F-002 Context

The codebase exposed two orchestration paths:

- a loader-oriented path centered around `engine::Loader`
- a Phase 3 pipeline-oriented path centered around the input → identify → decode → present flow

This duality introduced ambiguity about the canonical execution model.

### F-002 Evidence

- `engine::pipeline` orchestrates the Phase 3 semantic flow:
  - `InputSource`
  - `InputIdentifier`
  - `InputDecoder`
  - `normalize_decoded_content()`
  - `OutputPresenter`
  - `VfsDirectory`
- `engine::preview` and `engine::inspect` use the pipeline path rather than `Loader`
- the two paths operate over different intermediate models (`VirtualFile` / `GameImage` vs decoded/normalized pipeline models and `VfsDirectory`)

### F-002 Why it matters

Retaining two top-level orchestration models makes the system harder to understand, harder to extend, and harder to clean up safely.

It also creates uncertainty about where new behavior should be added and weakens the architectural boundaries established in Phase 3.

### F-002 Options

- Keep both paths and document them as distinct long-term layers
- Treat the pipeline as canonical and `Loader` as transitional
- Migrate configured/runtime execution onto the Phase 3 pipeline and remove `Loader`

### F-002 Decision

Resolved by D-002. The loader-based orchestration path has been removed, and Retromount now uses the Phase 3 pipeline as its sole orchestration model.

Implementation was performed on branch: `feature/pipeline-orchestration-consolidation`.

---

## F-003: Presenter and encoder responsibilities are not yet sharply defined

**Status:** Open  
**Issue:** [#43](https://github.com/lloydsmart/retromount/issues/43)

### F-003 Context

The current output side of the architecture includes both presenter and encoder concepts, but the boundary between them is not yet fully clear.

In particular, naming, output structure, and representation logic may currently be split across these layers in ways that are harder to reason about than necessary.

### F-003 Evidence

- the project contains a presenter abstraction and a concrete `GenericPresenter`
- the project contains an encoder abstraction and a concrete `BasicEncoder`
- output concerns such as file naming, output layout, and content representation appear to involve both layers
- future Phase 4 work is expected to expand output/view behavior, increasing the importance of a clean separation here

### F-003 Why it matters

If presenter and encoder responsibilities are unclear, future view work will be harder to implement cleanly.

This affects:

- where output structure should be decided
- where file naming should be decided
- where content representation/translation should be decided
- whether future output customizations can be added without modifying core logic

A weak boundary also makes plugin-style extensibility more difficult, because it becomes unclear which interface a new output behavior should target.

### F-003 Options

- Define presenters as responsible for structure and encoders as responsible for representation/naming details
- Define presenters as pure view builders and move most filename/output translation logic into encoders
- Collapse the distinction if both abstractions are not providing enough independent value

### F-003 Decision

TBD

---

## F-004: The core model boundary needs review to ensure `Content` and `GameContent` remain presentation-agnostic

**Status:** Resolved  
**Issue:** [#44](https://github.com/lloydsmart/retromount/issues/44)  
**Resolved by:** D-004

### F-004 Context

The current core model represents the project's normalized understanding of games and content, and must remain independent from output/view decisions.

This finding focused on ensuring that `Content`, `GameContent`, and `GamePart` describe semantic content only, without carrying representation-specific details such as filenames.

### F-004 Evidence

- Phase 3 introduced stronger normalization around `Content` and `GameContent`
- the system distinguishes between internal normalized content and output presentation
- ROM-related filename data had existed in normalized model types (`RomContent` and `RomPart`)
- output behavior such as filenames and display naming is now handled outside the core model, based on source information and encoder logic

### F-004 Why it matters

The core model should represent what the content *is*, not how it should be shown to a specific consumer.

If presentation-specific assumptions leak into `Content` or `GameContent`, then:

- alternative views become harder to implement
- internal semantics become tied to current output choices
- future plugin boundaries become less stable
- refactoring output behavior becomes more invasive than necessary

### F-004 Options

- Confirm the current model is sufficiently presentation-agnostic and document the boundary clearly
- Move presentation-oriented assumptions out of the core model and into presenter/encoder layers
- Introduce clearer intermediate types if the current model is carrying too many responsibilities

### F-004 Decision

Resolved by D-004.

Filename-oriented data has been removed from normalized ROM model types, and ROM naming/metadata presentation is now derived from source information at the encoder and inspection/CLI layers.

The normalized core model now carries semantic information only.

---

## F-005: The boundary between decoded content and normalized playable content needs clarification

**Status:** Resolved  
**Issue:** [#53](https://github.com/lloydsmart/retromount/issues/53)  
**Resolved by:** D-005

### F-005 Context

Retromount previously used a single `Content` model to represent both decoded input artifacts and normalized semantic output.

This meant the same top-level model family spanned multiple stages of the pipeline:

- decode produced `Content`
- normalize consumed and transformed `Content`
- present consumed normalized `Content`

While workable, this created ambiguity about whether all `Content` values were equally valid at every post-decode stage.

In particular, some variants represented pre-normalized artifacts (`Rom`, `Disc`, `Bytes`, `Text`), while others represented normalized semantic entities (`Game`).

### F-005 Evidence

- `InputDecoder` previously produced `Content`
- `normalize_content()` previously consumed `Vec<Content>` and produced `Vec<Content>`
- `GenericPresenter` previously assumed it should only receive normalized playable content and treated `Content::Rom` / `Content::Disc` as unreachable
- `BasicEncoder` similarly treated `Content::Rom` / `Content::Disc` as unreachable in presentation-time encoding
- parts of the pipeline relied on convention rather than type-level distinction to know whether content was decoded or normalized

### F-005 Why it matters

This weakened the stage boundary between decode and normalize.

If decoded content and normalized content share the same broad model without clearer separation, then:

- stage contracts are easier to violate accidentally
- presenter/encoder code must defend against states that “should not happen”
- future plugin boundaries become less explicit
- pipeline reasoning becomes more dependent on convention than on declared types

### F-005 Options

- Keep the current `Content` model and document the stage contract more explicitly
- Introduce a clearer distinction between decoded content and normalized content
- Introduce a dedicated normalized/playable model for post-normalization pipeline stages
- Strengthen type/API boundaries so later stages cannot receive pre-normalized variants accidentally

### F-005 Decision

Resolved by D-005.

Retromount now uses an explicit type separation between decoded content and normalized content:

- decode produces `DecodedContent`
- normalize consumes `DecodedContent` and produces `NormalizedContent`
- present consumes `NormalizedContent`
- encode consumes `NormalizedContent`

This makes the decode → normalize → present pipeline contract explicit in the type model and removes impossible pre-normalized variants from post-normalization stage boundaries.
