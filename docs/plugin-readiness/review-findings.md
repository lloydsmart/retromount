# Plugin Readiness Findings

This document tracks architectural findings related to plugin readiness.

---

## F-101: Pipeline component construction is duplicated

Pipeline components (`InputIdentifier`, `InputDecoder`, `OutputPresenter`, encoder)
are constructed directly in multiple application entrypoints:

- `main.rs`
- `engine/preview.rs`
- `engine/inspect.rs`

### Impact of F-101

- No single composition boundary exists
- Hard to swap implementations globally
- Increases risk of divergence between entrypoints

---

## F-102: Default presenter/encoder composition is hardcoded

The default composition of `GenericPresenter` with `BasicEncoder` is constructed
directly at the application edge.

### Impact of F-102

- Presenter/encoder pairing is not configurable
- Prevents alternate presenters from being introduced cleanly
- Couples application logic to specific implementations

---

## F-103: Extension points exist but are not surfaced through composition

Traits exist for:

- `InputIdentifier`
- `InputDecoder`
- `OutputPresenter`
- `OutputEncoder`

However, there is no centralized mechanism for supplying implementations.

### Impact of F-103

- System is extensible in theory but not in practice
- Application code depends on concrete types rather than interfaces

---

## F-104: Presenter/encoder relationship may be too rigid

Current design assumes a presenter is constructed with a single encoder.

### Impact of F-104

- Future presenters (e.g. MiSTer, Batocera) may require:
  - Multiple encoding strategies
  - More flexible encoding services
- Current structure may need to evolve to support richer composition

---

## F-105: No explicit composition boundary for pipeline execution

The pipeline accepts trait objects, but there is no explicit
`PipelineComponents` abstraction.

### Impact of F-105

- No clear boundary between engine and application composition
- Harder to introduce plugin systems later
