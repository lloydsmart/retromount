# Plugin Readiness Cleanup Tracker

This document tracks concrete implementation work for plugin readiness.

---

## High priority (composition boundary)

### F-101: Centralize pipeline component construction

- [ ] Introduce PipelineComponents struct
- [ ] Add default_pipeline_components() factory
- [ ] Export composition module from engine

---

### F-102: Remove hardcoded construction from entrypoints

- [ ] Update main.rs to use centralized components
- [ ] Update engine/preview.rs to use centralized components
- [ ] Update engine/inspect.rs to use centralized components

---

### F-103: Ensure pipeline only depends on trait interfaces

- [ ] Verify pipeline does not construct concrete implementations
- [ ] Confirm all stages use trait objects only

---

## Medium priority (composition flexibility)

### F-104: Review presenter/encoder relationship

- [ ] Evaluate whether presenters should depend on:
  - Single encoder
  - Multiple encoders
  - Encoding service abstraction
- [ ] Document future direction

---

### F-105: Define application-level composition boundary

- [ ] Ensure all entrypoints use PipelineComponents
- [ ] Remove duplicate wiring logic

---

## Low priority (future plugin support)

- [ ] Define plugin registration model (compile-time)
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
