# Cleanup Tracker

## High priority

- [x] Audit `src/input` vs `src/inputs`
- [x] Identify canonical orchestration path
- [x] Remove obsolete transitional helpers from Phase 3

### F-002 migration checklist

- [x] Audit `Loader` responsibilities
- [x] Identify all `Loader` call sites
- [x] Map each `Loader` responsibility to a pipeline-aligned destination
- [x] Rework configured/runtime execution to use pipeline output
- [x] Remove `Loader`
- [x] Remove dead `InputRegistry` / `VirtualFile`-only orchestration code if obsolete
- [x] Update architecture docs and findings

## Medium priority

- [ ] Review naming consistency for presenter/view/output terminology
- [ ] Review registry composition and default registration
- [ ] Remove dead code and unused helpers

## Low priority

- [ ] Add diagrams
- [ ] Improve module-level docs
