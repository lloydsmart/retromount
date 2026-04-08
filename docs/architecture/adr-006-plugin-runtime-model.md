# ADR-006 — Plugin Runtime Model

## Status

Proposed

---

## Context

Phase 5 introduces runtime extensibility to Retromount, allowing external plugins to contribute presenters, encoders, and related capabilities without requiring recompilation of the core binary.

This represents a significant architectural shift:

> Retromount transitions from a statically composed application into a runtime-extensible platform.

To support this, the system must define how plugins are:

* packaged
* discovered
* loaded
* executed
* isolated from the host
* versioned and validated for compatibility

The plugin runtime model must align with the architectural principles established in earlier phases:

* strict pipeline boundaries
* presentation-agnostic core model
* clear Presenter / Encoder separation
* deterministic and inspectable behaviour

---

## Decision Drivers

The chosen plugin runtime model must balance the following concerns:

### Safety and isolation

Plugins should not be able to crash or corrupt the host process, or violate architectural boundaries.

### Runtime extensibility

Plugins must be installable and usable without recompiling Retromount.

### Versioning and compatibility

The system must tolerate independent versioning of host and plugins without fragile ABI coupling.

### Implementation complexity

The initial implementation should be achievable within Phase 5 without excessive infrastructure overhead.

### Performance

The runtime model should support efficient processing of potentially large content sets.

### Plugin author experience

It should be reasonably straightforward to develop and distribute plugins.

### Observability and debugging

Failures should be visible, diagnosable, and attributable to specific plugins.

### Long-term ecosystem viability

The model should support future expansion, including third-party plugins and potentially multiple implementation languages.

---

## Options Considered

### Option A — Native Shared Libraries (C ABI)

Plugins are compiled as shared libraries and loaded into the Retromount process via a stable C ABI boundary.

#### Pros (Native shared libraries)

* High performance (no IPC or sandbox overhead)
* Simple execution model
* Familiar pattern for systems programming

#### Cons (Native shared libraries)

* Requires strict ABI stability across versions
* Weak isolation — plugin crashes can terminate the host
* Memory safety risks cross the boundary
* Difficult to evolve safely over time
* Strong coupling between host and plugin implementations

---

### Option B — WebAssembly Plugins

Plugins are compiled to WebAssembly and executed inside an embedded runtime.

#### Pros (WebAssembly plugins)

* Strong sandboxing and isolation
* Portable across platforms
* Explicit host/plugin interface
* Safer execution model than native libraries

#### Cons (WebAssembly plugins)

* Additional runtime complexity
* Performance overhead compared to native execution
* Requires explicit bridging for filesystem, I/O, and data access
* More complex plugin authoring model

---

### Option C — Out-of-Process Plugins

Plugins run as separate processes and communicate with the host via a defined protocol (e.g. IPC or stdio).

#### Pros (Out-of-process plugins)

* Strong isolation — plugin crashes do not affect the host
* No ABI compatibility concerns
* Language-agnostic plugin development
* Clear and explicit boundary between host and plugin
* Flexible versioning and deployment

#### Cons (Out-of-process plugins)

* IPC overhead
* Requires protocol design and implementation
* More complex execution model (process lifecycle, communication)
* Streaming and data transfer must be carefully designed

---

## Decision

Retromount will adopt **out-of-process plugins** as the initial runtime plugin model for Phase 5.

---

## Rationale

Out-of-process plugins provide the best balance of:

* safety
* flexibility
* long-term maintainability
* ecosystem viability

This model:

* avoids fragile ABI coupling between host and plugin
* prevents plugin failures from crashing the host
* enables plugins to be written in multiple languages
* creates a clean and explicit architectural boundary
* supports independent versioning of plugins and host

While this approach introduces additional complexity (protocol design, IPC), that complexity is **intentional and beneficial**, as it enforces clear separation of concerns and supports long-term extensibility.

---

## Consequences

### Positive

* Strong isolation guarantees between host and plugins
* Clear contract boundary encourages stable, well-defined interfaces
* Simplifies future support for third-party plugins
* Enables flexible packaging and distribution strategies
* Reduces risk of subtle memory or ABI-related bugs

### Negative

* Requires definition of a plugin communication protocol
* Introduces IPC overhead
* Increases implementation complexity in Phase 5
* Requires explicit handling of streaming and large data transfer
* Debugging may involve cross-process tracing

---

## Implications for Phase 5

### Phase 5A — Presentation Planning

* Must produce a **serialisable presentation plan**
* Plan must be transferable across process boundaries

### Phase 5B — Capability Resolution

* Capability metadata must be queryable from plugins
* Resolution logic remains in the host

### Phase 5D — Plugin Runtime Implementation

* Must implement:

  * plugin discovery
  * process lifecycle management
  * communication protocol
  * capability querying
  * artifact execution requests

---

## Follow-On Work

* Define plugin communication protocol (request/response model)
* Define plugin manifest format
* Implement plugin discovery mechanism
* Implement host-side plugin manager
* Implement at least one reference plugin
* Define logging and observability strategy across process boundaries

---

## Future Considerations

This decision does not prevent future support for additional plugin models.

Potential future expansions:

* WebAssembly plugins for sandboxed in-process execution
* Native plugins for high-performance scenarios
* Hybrid models depending on plugin type

However, all future models must conform to the same logical contract defined in this phase.

---

## Summary

Retromount will implement runtime extensibility using an out-of-process plugin model.

This establishes a robust, safe, and flexible foundation for:

* ecosystem-specific integrations
* independently distributed plugins
* long-term platform evolution

This decision prioritises architectural integrity and ecosystem viability over minimal implementation complexity.
