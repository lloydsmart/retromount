# Phase 4C — Policy Layer

This document defines the implementation plan and scope for Phase 4C.

Phase 4C builds on the Phase 4B presenter model and introduces a formal policy layer to control naming, formatting, and conflict behaviour without collapsing existing architectural boundaries.

---

## Goal

Introduce a dedicated policy layer that governs naming and related behavioural rules across presenters and encoders.

Specifically:

> The same normalized content and presentation structure can be rendered using different naming, formatting, and conflict-handling rules via interchangeable policy implementations.

---

## Scope

Phase 4C introduces:

- explicit policy support in runtime and pipeline execution
- policy abstractions for naming, formatting, and conflict resolution
- default policy implementations that preserve current behaviour
- the ability to vary naming and related rules independently of presenter and encoder selection

---

## Non-Goals

Phase 4C explicitly does **not** include:

- plugin loading or external extension mechanisms (Phase 4D)
- metadata enrichment or scraping
- redesign of the normalized core model
- changes to the presenter/encoder architectural boundary
- broad deduplication strategy redesign unless it falls naturally out of policy insertion points
- user-facing configuration formats beyond what is needed to establish the internal architecture

---

## Why This Phase Exists

Phase 4B established that presentation structure should be owned by presenters, while file materialization should be owned by encoders.

However, a further concern remains:

- how names are derived
- how names are normalized or sanitized
- how collisions are resolved
- how future compatibility profiles can alter these rules without changing structure or encoding behaviour

Without a dedicated policy layer, these concerns would leak into presenter and encoder implementations, making them harder to reuse, harder to swap, and harder to evolve.

Phase 4C exists to give these rules a proper home.

---

## Architectural Position

Retromount’s canonical pipeline remains:

1. Input
2. Identify
3. Decode
4. Normalize
5. Present
6. Encode
7. Materialize via VFS / mount adapter

Phase 4C adds policy as a cross-cutting rules layer that influences presentation and encoding without owning either concern.

Conceptually:

```text
Normalized -> Presenter -> Encoder -> VFS
        ^            ^
        |            |
      Policy influences naming, formatting, and conflict behaviour
```

Policy must not:

- define structure directly
- mutate normalized content
- replace presenter responsibilities
- replace encoder responsibilities

---

## Success Criteria

- policy abstractions exist for naming, formatting, and conflict handling
- default policy implementations preserve existing Phase 4B behaviour
- policy is threaded through runtime and pipeline execution
- at least one naming decision currently embedded in presenter or encoder logic is moved into policy
- the same normalized input and presenter can produce different output names under different policy implementations
- existing presenter and encoder boundaries remain intact

---

## Policy Responsibilities

Phase 4C establishes policy as the home for rules such as:

### Naming

Examples:

- game names
- part names
- playlist names
- platform label formatting where applicable

Examples of variation:

- `Final Fantasy VII (Disc 1).cue`
- `Final Fantasy VII - CD1.cue`
- `FF7_Disc1.cue`

### Formatting

Examples:

- whitespace normalization
- invalid character handling
- filesystem-safe transformations
- case normalization where required

### Conflict Handling

Examples:

- append numeric suffixes
- preserve first entry
- error on collision
- future strategies for stricter compatibility profiles

---

## Boundary Rules

Phase 4C depends on keeping responsibilities clear.

### Presenter owns

- what logical entries exist
- grouping and layout decisions
- directory structure
- whether artifacts such as playlists are part of the presentation

### Encoder owns

- how logical entries are materialized
- output representation
- inline file generation
- file backing choice

### Policy owns

- naming rules
- formatting rules
- conflict-resolution rules

A presenter may decide that a playlist exists.

A policy decides what it should be called.

An encoder decides how that playlist becomes a file and what content it contains.

---

## Proposed Implementation Plan

### Step 1 — Introduce policy abstractions

Add internal abstractions for:

- `NamingPolicy`
- `FormattingPolicy`
- `ConflictPolicy`
- `PolicySet`

This step should define the architecture only and should not change visible behaviour.

### Step 2 — Add default policy implementations

Create default policies that mirror the current naming and formatting behaviour already embedded in the system.

This provides a safe compatibility baseline.

### Step 3 — Thread policy through runtime and pipeline

Ensure that the execution path can supply policy to the components that need it.

Policy should be available to presenter and encoder code paths through explicit runtime context rather than hidden global state.

### Step 4 — Move naming decisions into policy

Identify existing hard-coded naming logic and migrate it into `NamingPolicy`.

Initial targets should include:

- file names
- part names
- playlist names
- other obvious naming hotspots currently embedded in presenter or encoder implementations

### Step 5 — Apply formatting policy separately

Introduce formatting as a distinct concern from naming.

This allows a raw logical name to be derived first, then normalized for output constraints.

### Step 6 — Introduce conflict handling at namespace boundaries

Apply conflict resolution where sibling names are inserted into the same logical namespace.

This must remain a naming-resolution concern and must not alter presentation structure.

### Step 7 — Prove swappability

Add at least one alternate policy implementation that changes output naming without requiring presenter or encoder changes.

This is the proof that policy is a real abstraction rather than a relocation of helper functions.

---

## Recommended First Slice

The first implementation slice should be intentionally conservative.

It should include:

- policy traits and `PolicySet`
- default implementations
- runtime and pipeline wiring
- tests proving default behaviour matches the current system

It should not yet attempt broad naming refactors beyond what is needed to establish the abstraction.

---

## Testing Expectations

Phase 4C should include:

### Unit tests

For default policy behaviour, including:

- game naming
- part naming
- playlist naming
- formatting behaviour
- conflict resolution behaviour

### Integration tests

To verify that:

- default policy preserves existing output
- alternate policy changes naming without changing structure

### Regression coverage

Existing preview, inspect, and mount-oriented behaviour should continue to behave as expected under the default policy set.

---

## Future Relevance

Phase 4C lays the groundwork for later extensibility.

In particular, it enables future support for:

- multiple naming styles
- compatibility profiles for specific consumers
- profile-based composition of presenter, encoder, and policy
- plugin-based policy implementations in later phases

This is especially important for long-term ideas where the same underlying data may need to conform to different external naming conventions without changing the core pipeline.

---

## Exit Condition

Phase 4C is complete when policy is a first-class architectural concept in the codebase, default behaviour is preserved, and naming-related rules can vary independently of presenter and encoder implementations.
