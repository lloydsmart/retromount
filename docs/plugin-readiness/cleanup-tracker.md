# Plugin Readiness Cleanup Tracker

This document tracks concrete implementation work for plugin readiness.

> Issue references link to GitHub for full context and discussion.

---

## High priority (composition boundary)

### F-101: Centralize pipeline component construction ([#58](https://github.com/lloydsmart/retromount/issues/58))

- [x] Introduce PipelineComponents struct
- [x] Add default_pipeline_components() factory
- [x] Export composition module from engine

---

### F-102: Remove hardcoded construction from entrypoints ([#59](https://github.com/lloydsmart/retromount/issues/59))

- [x] Update main.rs to use centralized components
- [x] Update engine/preview.rs to use centralized components
- [x] Update engine/inspect.rs to use centralized components

---

### F-103: Ensure pipeline only depends on trait interfaces ([#60](https://github.com/lloydsmart/retromount/issues/60))

- [x] Verify pipeline does not construct concrete implementations
- [x] Confirm all stages use trait objects only

---

## Medium priority (composition flexibility)

### F-104: Review presenter/encoder relationship ([#61](https://github.com/lloydsmart/retromount/issues/61))

- [ ] Evaluate whether presenters should depend on:
  - Single encoder
  - Multiple encoders
  - Encoding service abstraction
- [ ] Document future direction

---

### F-105: Define application-level composition boundary ([#62](https://github.com/lloydsmart/retromount/issues/62))

- [x] Ensure all entrypoints use PipelineComponents
- [x] Remove duplicate wiring logic

---

## Low priority (future plugin support)

- [ ] Define plugin registration model (compile-time) ([#63](https://github.com/lloydsmart/retromount/issues/63))
- [ ] Explore dynamic loading approach (.so/.dll)
- [ ] Define stable plugin API surface
- [ ] Add examples of alternate presenters

---

## Completed decisions

- [x] D-101: Introduce centralized pipeline composition boundary
- [x] D-102: Use compile-time composition as initial approach
- [x] D-103: Keep normalization non-pluggable
- [x] D-104: Treat presenters as primary output extension point
- [x] D-105: Avoid rigid presenter/encoder pairing assumptions
