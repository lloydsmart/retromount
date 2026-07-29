# Phase 6A: Presentation Migration Plan

## Purpose

Phase 6A begins the transition from **presenters as Rust implementations** to **presentations as declarative data**.

This follows:

* ADR-007: Declarative Presentation Specifications
* ADR-008: Runtime Encoder Plugins as the Primary Output Materialization Mechanism
* ADR-009: Defer Input Plugin Architecture

The goal is to preserve the existing output pipeline shape:

1. normalized content
2. presentation planning
3. capability resolution
4. artifact materialization

while replacing bespoke presenter implementations with a generic compiler over presentation specifications.

---

## Current situation

The current output-side design still reflects the earlier presenter model.

Notable components:

* `src/output/flat_presenter.rs`
* `src/output/grouped_presenter.rs`
* `src/output/present.rs`
* `src/output/presenter_registry.rs`
* presenter-oriented bootstrap and CLI wiring in `src/engine/*` and `src/main.rs`

These files currently mix two different concerns:

1. **presentation intent**

   * desired structure
   * grouping rules
   * naming choices
   * artifact requirements
   * companion artifact behavior

2. **plan compilation mechanics**

   * iterating normalized content
   * constructing `PresentationPlan`
   * building `PlanEntry`, `PlanDirectory`, and `PlanFile`
   * name allocation and conflict handling
   * artifact request construction
   * directory tree insertion
   * generated artifact wiring

Phase 6 should separate these concerns.

---

## Architectural direction

Retromount should move toward this model:

* **`PresentationSpec`** describes output intent as data
* **`PresentationCompiler`** interprets `PresentationSpec` into `PresentationPlan`
* **encoder plugins** continue to materialize requested artifacts through capability resolution

This means:

* adding a new target should require defining a new spec value or config
* adding a new target should **not** require adding a new Rust presenter implementation
* output behavior should be inspectable as data rather than hidden in bespoke code paths

---

## Presenter review

### `flat_presenter.rs`

`flat_presenter.rs` currently contains both presentation semantics and generic compilation logic.

#### Presentation intent currently embedded in `flat_presenter.rs`

* all emitted entries live at root
* `Game`, `Bytes`, and `Text` are all eligible for emission
* single-part games emit one file
* multi-disc games emit disc files plus a playlist
* `Bytes` request `ContentType::Bytes + Format::Bin`
* `Text` request `ContentType::Text + Format::Text`
* ROM games request `ContentType::Rom + Format::Bin`
* disc games request `ContentType::Disc + Format::Bin`
* playlists request `ContentType::Playlist + Format::M3u`
* names are derived from policy naming rules or helper conventions

This belongs in declarative presentation specifications.

#### Compilation mechanics currently embedded in `flat_presenter.rs`

* iterating `NormalizedContent`
* branching on `NormalizedContent::{Game, Bytes, Text}`
* maintaining `root_names`
* applying `policy.resolve_name_conflict(...)`
* constructing `ArtifactId`
* constructing `ArtifactRequest`
* constructing `PlanFile`
* wrapping source-backed artifacts with `SourceArtifact::single(...)`
* wrapping generated playlist artifacts with `GeneratedArtifact::Playlist(...)`
* assembling `PresentationPlan`

This belongs in generic presentation compilation logic.

#### Legacy or redesign candidates in `flat_presenter.rs`

* `bytes_file_name(...)` hard-codes `.bin`
* `text_file_name(...)` hard-codes `.txt` when no extension exists
* `estimate_disc_size(...)` is currently stubbed
* direct imperative branching is not the long-term extension model

These behaviors should be treated as migration references, not final architecture.

---

### `grouped_presenter.rs`

`grouped_presenter.rs` also mixes presentation intent and generic compilation machinery.

#### Presentation intent currently embedded in `grouped_presenter.rs`

* games are grouped under `platform/game/`
* platform naming is policy-driven
* game directory naming is policy-driven
* single-part games emit one file in the game directory
* multi-disc games emit disc files plus playlist in the game directory
* bytes/text preserve parent path structure derived from content IDs
* bytes and text use specific naming conventions
* artifact requests follow the same broad content-type/format patterns as flat

This belongs in declarative presentation specifications.

#### Compilation mechanics currently embedded in `grouped_presenter.rs`

* building a mutable directory tree
* descending and creating directories
* enumerating child names
* allocating distinct file and directory names
* normalizing and splitting paths
* inserting `PlanEntry::Directory(...)`
* inserting `PlanEntry::File(...)`
* constructing artifact requests and generated playlist artifacts

This belongs in reusable presentation compiler support.

#### Legacy or redesign candidates in `grouped_presenter.rs`

* deriving bytes/text path structure directly from IDs as fixed behavior
* helper naming heuristics for bytes/text
* unconditional structural decisions baked into code
* `estimate_disc_size(...)` stub

These are useful references but should not be promoted blindly into the new model.

---

## What should move where

### Move into `PresentationSpec`

The declarative “what”:

* layout shape
* grouping and placement rules
* match rules over normalized content
* naming rules
* artifact requirements
* companion artifact rules such as playlists
* future conditional emission rules

### Move into generic presentation compilation

The imperative “how”:

* traversal over normalized content
* evaluation of spec rules
* tree and path insertion
* name allocation and conflict handling
* file and directory creation
* artifact request construction
* generated artifact wiring
* `PresentationPlan` assembly

### Retire or replace

The current extension model:

* `FlatPresenter`
* `GroupedPresenter`
* `OutputPresenter` as the long-term extension mechanism
* `PresenterRegistry`

These may remain temporarily during migration, but they do not fit the Phase 6 end state.

---

## Components likely to remain valuable

These still fit the target architecture well and should generally remain:

* `src/output/plan.rs`
* `src/output/capabilities.rs`
* `src/output/resolution.rs`
* `src/output/materialize.rs`
* plugin protocol/runtime/discovery code
* policy system
* selected helpers in `src/output/presentation_expansion.rs`

The most important preserved boundary is:

* **`PresentationPlan`****remains the compilation target**

---

## Components to add

Phase 6A should introduce new output-side modules for the declarative model.

Suggested initial modules:

* `src/output/presentation_spec.rs`
* `src/output/presentation_compile.rs`

Possible support modules as the design grows:

* `src/output/presentation_support.rs`
* `src/output/presentation_tree.rs`
* `src/output/presentation_naming.rs`
* `src/output/presentation_catalog.rs`

These should stay close to the existing output planning and materialization code.

---

## Initial migration strategy

### Step 1: preserve current presenters as references

Do not delete `flat_presenter.rs` or `grouped_presenter.rs` first.

They should be kept temporarily as:

* behavior references
* regression oracles
* extraction sources for first declarative concepts

### Step 2: introduce a minimal internal `PresentationSpec`

The first version should be code-first, not YAML-first.

It should be able to express, at minimum:

* flat layout
* artifact requirements
* basic naming
* simple placement rules

### Step 3: implement a generic `PresentationCompiler`

This compiler should:

* take `PresentationSpec`
* operate only on normalized content
* produce `PresentationPlan`
* reuse existing policy and capability concepts
* avoid target-specific code paths

### Step 4: reproduce a simple current view through the spec model

Start with the simplest subset of existing behavior.

A good initial target is:

* flat root placement
* single-file emission for simple content
* artifact requests expressed declaratively

### Step 5: enrich only when a real view requires it

Only add model features when needed for concrete cases such as:

* grouped hierarchy
* multi-disc companion playlist behavior
* preserved source-relative paths
* PS2 / OPL requirements

This keeps the declarative model from turning into an ad hoc programming language.

### Step 6: replace presenter wiring with presentation-spec selection

Once the spec compiler is proven, migrate engine and CLI code away from:

* presenter names
* presenter registry
* hard-coded presenter construction

toward:

* presentation spec selection
* presentation compilation
* later, config-backed presentation loading

### Step 7: retire legacy presenter implementations

When flat and grouped behavior are reproducible via the generic compiler:

* remove `flat_presenter.rs`
* remove `grouped_presenter.rs`
* remove `presenter_registry.rs`
* remove presenter-specific bootstrap plumbing

Completed in Phase 6G after the Phase 6E parity work and Phase 6F runtime
migration established presentation specifications as the only built-in runtime
path.

---

## Immediate Phase 6A checklist

### Design

* [x] Define minimal `PresentationSpec` Rust types
* [x] Keep the first model deliberately small
* [x] Ensure specs operate only on normalized content
* [x] Ensure specs compile into `PresentationPlan`

### Extraction

* [x] Identify generic helpers that can move out of `grouped_presenter.rs`
* [x] Identify generic helpers that can move out of `flat_presenter.rs`
* [x] Separate naming conventions from mechanical plan assembly
* [x] Separate placement rules from tree insertion logic

### Compiler

* [x] Implement `compile_presentation_spec(...)`
* [x] Reuse existing policy conflict handling
* [x] Reuse existing capability requirement model
* [x] Keep encoder selection unchanged beneath the plan boundary

### Validation

* [x] Reproduce a simple flat view through `PresentationSpec`
* [x] Compare generated `PresentationPlan` against current presenter behavior
* [x] Add tests that prove the compiler is generic rather than target-specific
* [x] Confirm mount/inspect paths still work through the existing downstream pipeline

### Migration

* [x] Introduce presentation terminology alongside existing presenter terminology
* [x] Minimize disruption in CLI and config until the new path is proven
* [x] Remove legacy presenter registry only after equivalence is demonstrated

---

## Naming and terminology guidance

During migration, prefer these long-term terms in new code:

* `PresentationSpec`
* `PresentationCompiler`
* `presentation`
* `presentation catalog`

Avoid deepening the older model in new code:

* `OutputPresenter`
* `PresenterRegistry`
* target-specific presenter implementations

User-facing terms such as “view” may remain temporarily, but internally the architecture should move toward “presentation” rather than “presenter”.

---

## Key rule for Phase 6

A new output target should be added by defining a new presentation specification and, where necessary, a new encoder plugin.

A new output target should **not** require adding a new Rust presenter implementation.

That is the main architectural test for whether the migration is succeeding.
