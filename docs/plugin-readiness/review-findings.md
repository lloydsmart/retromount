# Architecture Review Findings

This document captures findings from the post-Phase 3 architecture review.

---

## F-101: Centralize pipeline component construction

**Status:** Completed  
**Issue:** [#58](https://github.com/lloydsmart/retromount/issues/58)

Pipeline components were previously constructed in multiple locations across the application, leading to duplication and inconsistent wiring.

This has been resolved by introducing a centralized composition boundary via `PipelineComponents` and `default_pipeline_components()`.

---

## F-102: Remove hardcoded construction from entrypoints

**Status:** Completed  
**Issue:** [#59](https://github.com/lloydsmart/retromount/issues/59)

Entrypoints previously constructed pipeline components directly, coupling application logic to concrete implementations.

All entrypoints now consume the centralized composition boundary.

---

## F-103: Ensure pipeline depends only on trait interfaces

**Status:** Completed  
**Issue:** [#60](https://github.com/lloydsmart/retromount/issues/60)

The pipeline must not depend on concrete implementations in order to support substitution and future plugin systems.

The runtime pipeline now depends exclusively on trait interfaces:
- `InputIdentifier`
- `InputDecoder`
- `OutputPresenter`

---

## F-104: Review presenter and encoder relationship

**Status:** Open  
**Issue:** [#61](https://github.com/lloydsmart/retromount/issues/61)

The current design assumes a presenter is constructed with a single encoder.

Future output systems (e.g. MiSTer, Batocera) may require:
- Multiple encoding strategies
- Context-aware encoding
- More flexible composition models

This relationship needs to be reviewed to ensure it does not constrain future development.

---

## F-105: Define application-level composition boundary

**Status:** Completed  
**Issue:** [#62](https://github.com/lloydsmart/retromount/issues/62)

The application previously lacked a clearly defined composition boundary.

This has now been established via `PipelineComponents`, ensuring:
- All entrypoints share consistent wiring
- No duplicate construction logic exists
- The system is ready for alternate implementations

---

## F-106: Define plugin registration model (compile-time)

**Status:** Open  
**Issue:** [#63](https://github.com/lloydsmart/retromount/issues/63)

A mechanism is required to register alternate implementations of pipeline components at compile time.

This should enable:
- Extension of input handlers
- Swappable presenters
- Future plugin-style extensibility without runtime loading

This is a prerequisite for Phase 4 extensibility work.
