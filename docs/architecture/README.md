# Architecture Docs

This directory captures the current architectural shape of Retromount and the review work being carried out in `feature/architecture-boundary-review`.

## Documents

- `boundaries.md` — current architectural boundaries and intended stage contracts
- `review-findings.md` — findings discovered during review
- `decisions.md` — architectural decisions made during the review
- `cleanup-tracker.md` — concrete cleanup/refactor tasks arising from findings

## Review goals

- clarify layer boundaries
- identify confusing or redundant structures
- remove obsolete or transitional code where appropriate
- improve long-term extensibility without implementing a full plugin system yet
