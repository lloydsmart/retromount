# Plugin Readiness Decisions

This document records architectural decisions made during the plugin-readiness review.

---

## D-101: Introduce a centralized pipeline composition boundary

A `PipelineComponents` struct will define the concrete implementations used
by the pipeline.

### Rationale for D-101

- Provides a single composition boundary
- Decouples application entrypoints from concrete implementations
- Enables future replacement with plugin-provided components

---

## D-102: Use compile-time composition as the first milestone

Initial plugin-readiness will be achieved through compile-time composition.

### Rationale for D-102

- Avoids complexity of dynamic loading
- Establishes stable interfaces first
- Allows incremental evolution toward runtime plugins

---

## D-103: Keep normalization non-pluggable

Normalization remains an internal pipeline stage implemented as a function.

### Rationale for D-103

- Central to core domain semantics
- No immediate need for alternative implementations
- Reduces complexity during early plugin-readiness work

---

## D-104: Treat presenters as primary output extension point

Presenters define output structure and layout, and may internally use encoders.

### Rationale for D-104

- Aligns with target systems such as MiSTer, Batocera, and PS2
- Keeps output concerns grouped logically
- Allows presenters to evolve independently of engine

---

## D-105: Avoid locking into single presenter/encoder pairing

Application-level architecture must not assume:

- One presenter per system
- One encoder per presenter

### Rationale for D-105

- Future presenters may require multiple encoding strategies
- Avoids premature constraints on output design
