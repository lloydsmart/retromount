# Phase 5 — Runtime Plugins & Adaptive Presentations

This document defines the proposed scope and architectural direction for Phase 5.

Phase 5 is the point at which Retromount begins to fully realize its original vision:

> A platform that adapts existing ROM and disc collections into the formats and layouts expected by different ecosystems, without duplicating or permanently reorganizing the source library.

Phase 4 established the internal foundations for this:

* a clean pipeline
* a presentation-agnostic core model
* a strict Presenter/Encoder boundary
* policy-driven naming and formatting
* mountable virtual filesystem output

Phase 5 builds on that by introducing runtime-extensible plugins and multi-encoder presentation planning.

---

## Goal

Enable Retromount to load capability at runtime and compose it safely into the pipeline, so users can extend the system without recompiling the main binary.

Specifically:

> Retromount should be able to discover and use separately installed plugins that contribute presenters, encoders, and related capabilities at runtime.

This supports the long-term product vision of installing ecosystem-specific support independently, for example:

* `retromount-plugin-mister`
* `retromount-plugin-batocera`
* `retromount-plugin-chd`

---

## Core Architectural Shift

Phase 5 introduces two major shifts:

### 1. Retromount becomes runtime-extensible

Retromount should no longer be limited to capabilities compiled directly into the main binary.

Instead, it should support a plugin model in which externally shipped components can be installed and discovered at runtime.

### 2. Presentations may require multiple encoders

A presentation is not necessarily backed by a single encoder.

For example, a Batocera-oriented presentation might require:

* CHD encoding for optical disc content
* ZIP encoding for some ROM content
* passthrough encoding for already-compatible files

A different presentation such as MiSTer may require a different but overlapping set of encoders.

This means the architecture must support:

> one presentation plan being satisfied by multiple encoders

rather than assuming a single presenter/encoder pair.

---

## Design Principles

Phase 5 should preserve the architectural boundaries established in earlier phases.

### Pipeline integrity remains mandatory

Plugins must integrate into defined pipeline stages. They must not bypass or collapse the pipeline into a less structured model.

### The core model remains presentation-agnostic

Plugins must not reintroduce filenames, filesystem concerns, or consumer-specific semantics into normalized content.

### Presenter and Encoder responsibilities remain distinct

Presenters define the intended structure and artifact requirements.

Encoders materialize those artifacts.

A presenter must not directly own or hard-code concrete encoding behaviour.

### Runtime extensibility must be real

Phase 5 should not introduce a temporary “fake plugin” model that later needs to be discarded in favour of a true runtime mechanism.

---

## Scope

Phase 5 includes the following work.

### 1. Define a runtime plugin model

Retromount should define a formal plugin contract that supports runtime discovery and capability loading.

At minimum, this should cover:

* plugin identity
* plugin type
* supported capabilities
* compatibility/version metadata
* configuration surface
* error reporting expectations

This plugin model must be suitable for independently packaged plugins.

### 2. Introduce presentation planning

Presenters should evolve from directly producing encoded output toward producing a declarative presentation plan.

A presentation plan should describe:

* the intended logical output structure
* the files/artifacts required at specific output locations
* the source normalized content each artifact derives from
* the required or preferred encoding characteristics for each artifact

This allows the system to resolve each artifact independently against one or more available encoders.

### 3. Add encoder capability resolution

Retromount should introduce a resolution layer that matches presentation artifact requirements against available encoder capabilities.

This should support:

* multiple encoders contributing to one presentation
* deterministic selection behaviour
* clear failure modes when no suitable encoder is available
* optional preference and fallback rules

### 4. Implement runtime plugin loading

Retromount should support a real runtime loading mechanism for plugins.

The exact implementation mechanism remains open for design evaluation.

Candidate approaches include:

* native shared libraries with a stable ABI
* WebAssembly-based plugins
* out-of-process plugins with an explicit protocol

Phase 5 should choose one initial runtime model and implement it properly.

### 5. Add plugin discovery and inspection

Users should be able to inspect what plugins are installed and what capabilities they provide.

This includes functionality to:

* list installed plugins
* inspect plugin metadata
* inspect available presentations and encoders
* inspect resolution decisions for a planned presentation

### 6. Deliver at least one real external integration

Phase 5 should include at least one externally installable plugin that proves the model end-to-end.

Likely candidates:

* MiSTer presentation plugin
* Batocera presentation plugin
* CHD encoder plugin

### 7. Add configuration support for runtime capability selection

Retromount should support selecting presentations and constraining or influencing encoder resolution through CLI and/or configuration.

The model should reflect that multiple encoders may participate in a single presentation.

---

## Non-Goals

Phase 5 does not include:

* metadata scraping
* ROM management features
* artwork acquisition
* permanent export pipelines as a primary workflow
* GUI or web UI
* plugin marketplace/distribution infrastructure
* automatic downloading of plugins
* solving every possible security or sandboxing concern
* finalising every future plugin category

---

## Proposed Architectural Model

### Presenter output becomes declarative

Move from:

`normalized content -> presenter -> encoder -> VFS`

to:

`normalized content -> presenter -> presentation plan -> capability resolver -> multiple encoders -> VFS`

### Presentation plan responsibilities

A presentation plan should describe:

* directories to create
* logical files to expose
* normalized content backing each file
* required artifact representation (and constraints)
* optional preferences or constraints

### Conceptual Model (Non-Normative)

A presentation plan represents a declarative description of the intended output.

At a high level, it consists of:

* a virtual directory tree
* a set of artifact requests attached to file nodes

Each artifact request describes:

* the normalized content it derives from
* the desired output representation
* any constraints or preferences for encoding

For example (conceptually):

* `/PlayStation/Game Name/Game Name.chd`
  * source: DiscContent
  * requirement: representation=CHD

* `/NES/Game Name/Game Name.zip`
  * source: RomParts
  * requirement: representation=ZIP

* `/Arcade/Game Name/Game Name.rom`
  * source: SingleRom
  * requirement: representation=Passthrough

### Encoder capability model

Encoders advertise capabilities rather than existing as fixed pairings.

Examples:

* passthrough encoder → exposes existing files
* ZIP encoder → archive-based output
* CHD encoder → optical disc → CHD
* ISO encoder → optical disc → ISO

### Artifact Requirements vs Encoder Capabilities

A presentation plan expresses **artifact requirements**.

Encoders advertise **capabilities**.

Retromount is responsible for matching requirements to capabilities.

For example:

* requirement: "optical-disc → CHD"
* capability: "can encode optical-disc content to CHD"

A match occurs when an encoder can satisfy the requirement for a given artifact.

This separation ensures that:

* presenters remain declarative
* encoders remain interchangeable
* resolution logic is centralised and inspectable

### Resolution Behaviour (Initial Expectations)

The capability resolver should:

* select exactly one encoder per artifact
* behave deterministically given the same inputs and plugin set
* fail clearly if no suitable encoder is available
* optionally support preference rules (e.g. prefer passthrough)
* resolution should not depend on plugin load order

Initial implementations do not need to support:

* complex scoring systems
* cost-based optimisation
* multi-step encoding pipelines

These may be introduced in later phases.

### Plugin Contract (Initial Expectations)

Plugins should:

* declare their type (presenter, encoder, etc.)
* advertise capabilities or provided functionality
* declare compatibility with a host version range
* expose a clear entry point for invocation

Plugins should not:

* access internal Retromount state directly
* assume implementation details of the host
* bypass the defined pipeline stages
* assume exclusive ownership of an artifact or content type

### Failure Model (Initial Direction)

Failures during planning or resolution should be:

* explicit
* actionable
* tied to missing capabilities or incompatible plugins

For example:

* "No encoder available for optical-disc → CHD"
* "Plugin X is incompatible with this Retromount version"

Silent fallback behaviour should be avoided in initial implementations.

### No Implicit Fallbacks (Initial Constraint)

Phase 5 should avoid introducing implicit or hidden fallback behaviour.

If a required artifact cannot be satisfied, the system should fail clearly rather than attempting silent alternatives.

Explicit fallback strategies may be introduced later via policy or configuration.

---

## Plugin Runtime Options

### Option A: Native shared libraries (C ABI)

#### Pros (Native shared libraries)

* high performance
* package-manager friendly
* simple user model

#### Cons (Native shared libraries)

* requires stable ABI boundary
* weaker isolation
* crash risk propagates to host

---

### Option B: WebAssembly plugins

#### Pros (WebAssembly plugins)

* sandboxed
* portable
* structured host/plugin API

#### Cons (WebAssembly plugins)

* runtime complexity
* performance considerations
* explicit host bridging required

---

### Option C: Out-of-process plugins

#### Pros (Out-of-process plugins)

* strong isolation
* no ABI issues
* language-agnostic
* robust versioning

#### Cons (Out-of-process plugins)

* IPC overhead
* protocol complexity
* streaming considerations

---

## Initial Recommendation

### Critical Architectural Decision

The most important architectural change in Phase 5 is:

> presenters must emit a declarative plan that can be satisfied by multiple encoders

This decision is independent of the chosen plugin runtime mechanism and should be implemented first.

If this is done correctly:

* plugin runtime can evolve independently
* encoder implementations remain interchangeable
* ecosystem-specific presentations become straightforward to express

If this is done incorrectly:

* presentations will remain tightly coupled to encoders
* multi-encoder scenarios will be difficult or fragile
* future plugin support will be constrained

---

## CLI / UX Direction

```bash
retromount inspect <input> --presentation mister
retromount mount <input> <mountpoint> --presentation batocera
```

Optional controls:

```bash
--allow-encoder chd
--allow-encoder zip
--prefer-encoder passthrough
--plugin-dir /usr/lib/retromount/plugins
```

Diagnostics:

```bash
retromount plugins list
retromount plugins inspect mister
retromount plan inspect <input> --presentation batocera
```

---

## Suggested Sub-Phases

### Phase 5A — Presentation Planning Model

### Phase 5B — Capability Resolution

### Phase 5C — Plugin Runtime Selection

### Phase 5D — Plugin Runtime Implementation

### Phase 5E — External Plugin Proof

### Phase 5F — Packaging & UX

---

## Success Criteria

* runtime plugin discovery works
* presenters emit declarative plans
* multiple encoders satisfy one presentation
* resolution is deterministic and inspectable
* clear errors for missing capabilities
* at least one external plugin works end-to-end
* architectural boundaries remain intact

---

## Open Questions

1. Which runtime model to choose?
2. Plugin manifest format?
3. Version compatibility strategy?
4. Capability description model?
5. Plugin configuration approach?
6. Preference vs requirement handling?
7. Failure model?
8. Observability/debugging?
9. Sandbox requirements?
10. First integration target?

---

## Summary

Phase 5 transforms Retromount from:

> a system that processes ROMs

into:

> a platform that adapts ROM collections to any ecosystem

This is achieved through:

* runtime plugins
* declarative presentation planning
* multi-encoder resolution
* strict adherence to architectural boundaries

This phase is the foundation for everything that comes next.
