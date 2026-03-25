# Cleanup Tracker

## High priority

- [x] Audit `src/input` vs `src/inputs`
- [x] Identify canonical orchestration path
- [ ] Remove obsolete transitional helpers from Phase 3

### F-002 migration checklist

- [ ] Audit `Loader` responsibilities
- [ ] Identify all `Loader` call sites
- [ ] Map each `Loader` responsibility to a pipeline-aligned destination
- [ ] Rework configured/runtime execution to use pipeline output
- [ ] Remove `Loader`
- [ ] Remove dead `InputRegistry` / `VirtualFile`-only orchestration code if obsolete
- [ ] Update architecture docs and findings

## Medium priority

- [ ] Review naming consistency for presenter/view/output terminology
- [ ] Review registry composition and default registration
- [ ] Remove dead code and unused helpers

## Low priority

- [ ] Add diagrams
- [ ] Improve module-level docs
