# Cleanup Tracker

This document tracks architectural findings (F-XXX) and the concrete work required to resolve them.

---

## High priority (architecture boundary fixes)

### F-001: Input vs builtin_inputs naming ambiguity

- [x] Rename `src/inputs` → `src/builtin_inputs`
- [x] Update imports and module references
- [x] Update architecture documentation

---

### F-002: Loader vs pipeline orchestration ambiguity

- [x] Audit `Loader` responsibilities
- [x] Identify all `Loader` call sites
- [x] Map each responsibility to pipeline stages
- [x] Rework configured/runtime execution to use pipeline
- [x] Remove `Loader`
- [x] Remove dead `InputRegistry` / `VirtualFile` orchestration paths
- [x] Update architecture docs and findings

---

### F-003: Presenter vs encoder responsibility boundary

- [x] Move file naming into encoder
- [x] Remove duplicated naming logic from presenter
- [x] Introduce encoder support for game parts
- [x] Introduce encoder support for playlist generation
- [x] Move file materialization (inline vs source-backed) into encoder
- [x] Update presenter to consume fully-encoded outputs

---

### F-004: Core model leaking presentation concerns

- [x] Audit `GameContent`, `DiscPart`, `RomPart`, and `RomContent` for presentation-specific fields
- [x] Identify fields that exist only to support current presenter/CLI output
- [x] Define stricter core model responsibilities
- [x] Remove or relocate presentation-derived data where appropriate
- [x] Move ROM filename derivation out of normalized model types
- [x] Update documentation and findings

---

### F-005: Decoded content vs normalized content boundary ambiguity

**Issue:** [#53](https://github.com/lloydsmart/retromount/issues/53)

- [ ] Define decoded content model
- [ ] Define normalized content model
- [ ] Update decoder to produce decoded content only
- [ ] Update normalizer to transform decoded content into normalized content
- [ ] Narrow presenter interfaces to normalized content only
- [ ] Narrow encoder interfaces to normalized content only
- [ ] Remove impossible post-normalization variants from presenter/encoder paths
- [ ] Remove `unreachable!()` branches caused by over-broad content types
- [ ] Update architecture docs and findings
- [ ] Update tests for new stage-aligned types

## Medium priority (post-boundary cleanup)

- [ ] Review naming consistency for presenter / encoder / output terminology
- [ ] Review default encoder/presenter composition strategy
- [ ] Remove dead code and unused helpers after refactors
- [ ] Audit test coverage for new encoder boundary

---

## Low priority (documentation & polish)

- [ ] Add architecture diagrams (pipeline + boundaries)
- [ ] Improve module-level documentation
- [ ] Add examples for alternate encoders/presenters

## Completed decisions

- [x] D-001: clarify input vs builtin_inputs boundary
- [x] D-002: consolidate orchestration onto pipeline
- [x] D-003: enforce presenter/encoder separation
- [x] D-004: ensure core model remains presentation-agnostic
