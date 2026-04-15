# Phase 5 — Runtime Plugins & Adaptive Presentations

This document defines the scope and architectural direction for Phase 5.

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

## Status

### Implemented (Phases 5A–5E)

* Declarative presentation planning
* Capability-based encoder resolution
* Plugin protocol definition
* Out-of-process plugin runtime
* Plugin discovery and registration
* End-to-end plugin integration (validated via integration tests)

### Remaining (Phase 5F+)

* Plugin packaging and distribution model
* Plugin configuration surface
* CLI ergonomics and discovery tooling
* Additional real-world plugins (e.g. MiSTer, Batocera)
* Optional runtime improvements (e.g. WASM exploration)

See also: `docs/architecture/plugins.md` for the runtime plugin model.

---

## Goal

Enable Retromount to load capability at runtime and compose it safely into the pipeline, so users can extend the system without recompiling the main binary.

Specifically:

> Retromount should be able to discover and use separately installed plugins that contribute encoding capabilities at runtime.

This supports the long-term product vision of installing ecosystem-specific support independently, for example:

* `retromount-plugin-mister`
* `retromount-plugin-batocera`
* `retromount-plugin-chd`

---

## Core Architectural Shift

Phase 5 introduces two major shifts:

### 1. Retromount becomes runtime-extensible

Retromount is no longer limited to capabilities compiled into the main binary.

Instead, it supports a plugin model in which externally shipped components can be installed and discovered at runtime.

### 2. Presentations may require multiple encoders

A presentation is not necessarily backed by a single encoder.

For example, a Batocera-oriented presentation might require:

* CHD encoding for optical disc content
* ZIP encoding for some ROM content
* passthrough encoding for already-compatible files

This means the architecture must support:

> one presentation plan being satisfied by multiple encoders

rather than assuming a single presenter/encoder pair.

---

## Design Principles

Phase 5 preserves the architectural boundaries established in earlier phases.

### Pipeline integrity remains mandatory

Plugins integrate into defined pipeline stages and must not bypass or collapse the pipeline.

### The core model remains presentation-agnostic

Plugins must not introduce filenames, filesystem concerns, or consumer-specific semantics into normalized content.

### Presenter and Encoder responsibilities remain distinct

* Presenters define structure and artifact requirements
* Encoders (including plugins) materialize those artifacts

### Runtime extensibility must be real

The plugin model is implemented as a real runtime mechanism (out-of-process), not a temporary abstraction.

---

## Scope

Phase 5 includes the following work.

### 1. Define a runtime plugin model

A formal plugin contract is defined (see `plugins.md`) covering:

* plugin identity
* supported capabilities
* protocol compatibility
* error handling

### 2. Introduce presentation planning

Presenters now produce a **declarative presentation plan** describing:

* logical output structure
* required artifacts
* source content relationships
* encoding requirements

### 3. Add encoder capability resolution

A resolution layer matches artifact requirements to encoder capabilities.

Supports:

* multiple encoders per presentation
* deterministic selection
* explicit failure when unsatisfied

### 4. Implement runtime plugin loading

Phase 5 selects and implements:

> **Out-of-process plugins with a JSON protocol over stdin/stdout**

This provides:

* strong isolation
* language-agnostic plugin development
* stable versioning boundary

### 5. Add plugin discovery

Plugins are:

* discovered from a directory
* validated via manifest
* registered into the encoder registry

### 6. Deliver external integration proof

A fixture plugin demonstrates:

* manifest exchange
* capability registration
* materialization via plugin
* deterministic integration test coverage

### 7. Add configuration hooks

Initial CLI support includes:

* `--plugin-dir` for runtime loading
* future support for encoder constraints and preferences

---

## Non-Goals

Phase 5 does not include:

* metadata scraping
* ROM management features
* artwork acquisition
* plugin marketplace/distribution
* automatic plugin installation
* GUI or web UI
* full sandboxing model
* final plugin ecosystem design

---

## Proposed Architectural Model

### Presenter output is declarative

```text
normalized content
    ↓
presenter
    ↓
presentation plan
    ↓
capability resolver
    ↓
encoders (built-in + plugins)
    ↓
VFS
```

### Presentation plan responsibilities

A presentation plan describes:

* directory structure
* logical files
* normalized content backing
* encoding requirements

### Encoder capability model

Encoders (including plugins) advertise capabilities:

* content type
* supported formats
* supported features
* priority

### Artifact Requirements vs Capabilities

* Presenters define **requirements**
* Encoders define **capabilities**
* Resolver matches them deterministically

---

## Resolution Behaviour

The resolver:

* selects exactly one encoder per artifact
* is deterministic
* fails explicitly when no match exists
* does not depend on plugin load order

---

## Plugin Runtime Model

Phase 5 adopts:

> **Out-of-process plugins**

See `docs/architecture/plugins.md` for full details.

### Key properties

* process-per-invocation
* JSON protocol over stdin/stdout
* no shared state
* strong isolation

---

## Failure Model

Failures are:

* explicit
* actionable
* tied to missing capabilities or invalid plugins

No implicit fallbacks are introduced in Phase 5.

---

## CLI / UX Direction

```bash
retromount inspect <input> --presentation mister
retromount mount <input> <mountpoint> --presentation batocera
```

Optional controls:

```bash
--allow-encoder chd
--prefer-encoder passthrough
--plugin-dir ./plugins
```

---

## Suggested Sub-Phases

* 5A — Presentation Planning
* 5B — Capability Resolution
* 5C — Plugin Protocol
* 5D — Plugin Runtime
* 5E — Integration Proof
* 5F — Packaging & UX

---

## Success Criteria

* runtime plugin discovery works
* presenters emit declarative plans
* multiple encoders satisfy one presentation
* resolution is deterministic
* plugin integration works end-to-end
* architectural boundaries remain intact

---

## Summary

Phase 5 transforms Retromount from:

> a system that processes ROMs

into:

> a platform that adapts ROM collections to different ecosystems

This is achieved through:

* runtime plugins
* declarative presentation planning
* multi-encoder resolution

Phase 5 establishes the extensibility foundation for the system.
