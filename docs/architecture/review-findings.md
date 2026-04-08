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

## F-003: Presenter and encoder responsibilities were not sharply defined

**Status:** Resolved  
**Issue:** [#43](https://github.com/lloydsmart/retromount/issues/43)  
**Resolved by:** D-003

### F-003 Context

The output side of the architecture included both presenter and encoder concepts, but the boundary between them was not initially explicit.

In particular, naming, output structure, and representation logic were split across these layers in ways that were harder to reason about than necessary.

### F-003 Evidence

- the project contains a presenter abstraction and a concrete `GenericPresenter`
- the project contains an encoder abstraction and a concrete `BasicEncoder`
- output concerns such as file naming, output layout, and content representation were previously distributed across both layers
- future output/view work increased the need for a clean and enforceable separation

### F-003 Why it matters

If presenter and encoder responsibilities are unclear, output behaviour becomes harder to extend and reason about.

This affects:

- where output structure is decided
- where file naming is decided
- where content representation and transformation are decided
- whether alternate output customizations can be introduced without modifying core logic

A weak boundary also makes plugin-style extensibility more difficult, because it becomes unclear which interface new output behaviour should target.

### F-003 Options

- define presenters as responsible for structure and encoders as responsible for representation and naming details
- define presenters as pure view builders and move most filename/output translation logic into encoders
- collapse the distinction if both abstractions do not provide enough independent value

### F-003 Decision

Resolved by D-003.

Retromount retains both Presenter and Encoder abstractions with a strict responsibility boundary:

- Presenter owns structure, grouping, and layout
- Encoder owns naming, representation, file backing, and generated file content

This boundary is now enforced in code, with no remaining responsibility overlap.

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

---

## F-104: Presenter selection was hardcoded rather than registry-backed

**Status:** Resolved  
**Issue:** [#61](https://github.com/lloydsmart/retromount/issues/61)  
**Resolved by:** D-006

### F-104 Context

Retromount supported multiple presenters, but presenter selection was originally hardcoded in runtime composition.

This meant that although the output architecture had presenter abstractions, the system still relied on fixed built-in branching when choosing which presenter to use.

### F-104 Evidence

- presenter construction was previously performed via direct branching in pipeline component composition
- runtime flows selected presenters through hardcoded paths rather than registry-backed lookup
- configured and CLI-driven view selection depended on built-in selection logic rather than a first-class composition boundary

### F-104 Why it matters

Hardcoded presenter selection weakens extensibility.

If presenter selection is not registry-backed, then:

- adding new presenters requires modifying composition code directly
- runtime selection remains coupled to built-in implementations
- configuration cannot act as a clean composition surface
- future plugin-style presenter integrations become harder to support cleanly

### F-104 Options

- keep built-in presenter branching and document it as acceptable
- introduce presenter registry-backed selection while keeping built-ins static
- defer all presenter extensibility until a future plugin phase

### F-104 Decision

Resolved by D-006.

Retromount now resolves presenters via a registry-backed composition model rather than hardcoded branching.

This includes:

- `PresenterRegistry`
- registry-backed presenter validation and lookup
- removal of the legacy `PresenterKind` selection path
- configuration-driven presenter selection per view

---

## F-106: Presenter and encoder composition was implicit rather than explicit

**Status:** Resolved  
**Issue:** [#63](https://github.com/lloydsmart/retromount/issues/63)  
**Resolved by:** D-007

### F-106 Context

Retromount had separate presenter and encoder abstractions, but their composition was originally implicit.

In practice, encoder selection was hidden behind presenter construction, and presenter/encoder pairing was not exposed as an explicit runtime or configuration concern.

### F-106 Evidence

- presenters previously owned concrete encoder implementations directly
- encoder choice was effectively fixed inside presenter construction paths
- runtime composition selected presenters without exposing encoder selection explicitly
- configured execution did not initially allow presenter/encoder pairing to vary per view

### F-106 Why it matters

Implicit composition weakens the architectural boundary between structure and representation.

If presenter/encoder composition is not explicit, then:

- encoder choice remains hidden and harder to reason about
- composition cannot be configured cleanly per view
- future alternate encoders are harder to introduce safely
- extensibility work remains partial even when separate abstractions exist

### F-106 Options

- keep encoder selection implicit inside presenter construction
- make presenter/encoder composition explicit in the composition layer
- defer encoder composability until a later plugin phase

### F-106 Decision

Resolved by D-007.

Retromount now composes presenters and encoders explicitly through the composition layer.

This includes:

- `EncoderRegistry`
- explicit presenter/encoder construction in bootstrap and pipeline composition
- configuration-driven presenter/encoder selection per view
- removal of hidden built-in encoder coupling from presenter selection paths
