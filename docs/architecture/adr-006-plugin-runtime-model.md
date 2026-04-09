# ADR-006 — Plugin Runtime Model

**Status:** Accepted
**Date:** 2026-04-09
**Related:** Phase 5 — Runtime Plugins

---

## Context

Phase 5 introduces runtime extensibility to Retromount via plugins.

Retromount’s architecture has already established:

* a **normalized, presentation-agnostic core model**
* a strict **Presenter vs Encoder separation** (D-003)
* a **policy-driven presentation layer** (Phase 4C)
* a **host-controlled pipeline** from input → normalize → present → encode → VFS

Phase 5 extends this by allowing presenters and encoders to be implemented externally.

At the same time, Phase 5 introduces a key architectural shift:

> Presenters produce a **declarative presentation plan**, not encoded outputs.

This implies:

* the **host owns orchestration and capability resolution**
* plugins provide **capabilities**, not control flow
* multiple encoders may be used within a single presentation

Given this, the project must choose a **plugin runtime model**.

---

## Decision

Retromount will:

### 1. Adopt a declarative presentation-plan model

* Presenters emit a **presentation plan** describing desired artifacts
* The host resolves each artifact against available encoder capabilities
* Encoders materialize artifacts on demand
* The host retains full control of orchestration and resolution

---

### 2. Implement plugins as out-of-process components

* Plugins run as **separate executables**
* Communication occurs via an explicit **request/response protocol**
* The host is responsible for:

  * plugin discovery
  * capability aggregation
  * compatibility checks
  * deterministic resolution
  * diagnostics

---

### 3. Define a runtime-agnostic plugin contract

* The plugin interface is defined as a **logical protocol**, independent of transport
* The initial implementation uses **subprocess IPC**
* Future runtimes (e.g. WebAssembly) may implement the same contract

---

### 4. Explicitly reject in-process ABI plugins

Retromount will not support:

* dynamically loaded shared libraries (`.so`, `.dll`) as plugins
* Rust-to-Rust ABI coupling between host and plugins

---

## Rationale

### Declarative planning aligns with existing architecture

The presentation-plan model:

* preserves the Presenter/Encoder boundary (D-003)
* keeps the core model presentation-agnostic (D-004)
* enables multi-encoder resolution per presentation
* prevents presenters from embedding encoding logic

This is a natural continuation of Phase 3–4 architectural decisions.

---

### Out-of-process plugins provide strong isolation

* plugin failures do not crash the host
* safer for long-running operations (e.g. mounted filesystem)
* supports timeouts, error capture, and controlled execution

---

### Protocol boundary avoids ABI instability

Using an explicit protocol:

* avoids Rust ABI compatibility issues
* allows independent versioning of host and plugins
* supports backward/forward-compatible evolution

---

### Enables future plugin ecosystem

Out-of-process plugins:

* support third-party development
* allow implementation in other languages
* enable per-plugin packaging and distribution

---

### Aligns with future WebAssembly support

A protocol-based design:

* decouples the plugin contract from the runtime
* allows future implementation via WebAssembly without redesign
* avoids lock-in to a specific execution model

---

### In-process ABI plugins conflict with project goals

In-process plugins were rejected because they:

* tightly couple host and plugin implementations
* introduce ABI and versioning fragility
* allow plugin crashes to terminate the host
* hinder evolution toward sandboxed execution
* do not align with a capability-based architecture

---

## Consequences

### Positive

* clear architectural boundaries
* improved stability and fault isolation
* flexible evolution of plugin contract
* future-proof toward WASM or alternative runtimes
* strong foundation for diagnostics and introspection

---

### Negative

* increased implementation complexity (IPC, discovery, lifecycle)
* runtime overhead (process startup, serialization)
* need to define and maintain protocol schemas
* more complex packaging and distribution

---

### Neutral / Trade-offs

* performance is slightly reduced compared to in-process execution
* plugins cannot directly use internal Rust types
* contract design becomes a first-class concern

---

## Implementation Notes

### Plugin types

Two initial plugin categories:

* **Presenter plugins**

  * input: normalized content + policy/config
  * output: presentation plan

* **Encoder plugins**

  * advertise capabilities
  * accept artifact materialization requests

---

### Host responsibilities

The host must:

* discover plugins
* negotiate protocol/version compatibility
* collect and index encoder capabilities
* resolve artifacts deterministically
* provide clear diagnostics (e.g. “why this encoder was chosen”)

---

### Contract design principles

* request/response model (no shared memory)
* capability-based, not imperative control
* explicit versioning and feature negotiation
* structured error reporting
* minimal, stable surface area

---

### Runtime abstraction

The host should define an internal abstraction layer for plugins.

Initial implementation:

* subprocess-based adapter

Future implementations may include:

* WebAssembly runtime adapter

---

## Future Considerations

* WebAssembly-based plugin runtime for sandboxing
* plugin signing and trust model
* caching and performance optimisation
* streaming vs materialized artifact handling
* plugin capability prioritisation and user preferences

---

## Summary

Retromount adopts a **declarative, capability-driven plugin architecture** with **out-of-process plugins** and an **explicit protocol boundary**, enabling safe extensibility while preserving architectural integrity and future flexibility.
