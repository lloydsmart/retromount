# ADR-009: Defer Input Plugin Architecture Until After Declarative Presentations

## Status

Accepted

## Context

Retromount is expected to require extensibility on the input side as well as the output side. Future support may include:

* additional source or container mechanisms
* additional format decoders
* extensible handling for containers such as archives
* format-specific decoding for disc and ROM inputs

Examples discussed include ISO, CHD, and ZIP-based inputs. Importantly, these do not all belong to the same conceptual layer:

* ZIP is primarily a container/source concern
* ISO and CHD are format/decode concerns

This means “input plugins” are not a single flat category and likely require deliberate architecture across source/container and decoder responsibilities.

At the same time, Retromount is at a point where it must choose a clear next implementation focus. Declarative presentation specifications are the more immediate continuation of the current architecture because they build directly on the normalized-content-to-`PresentationPlan` output pipeline that already exists.

Attempting to design declarative presentations and input plugins in the same phase would open two significant architectural fronts at once.

## Decision

Retromount will **defer input plugin architecture until after Phase 6 declarative presentation work**.

Phase 6 will focus on declarative presentation specifications and real encoder plugin validation. Input extensibility will be addressed in a later phase with dedicated architectural work.

This means that:

* input extensibility is recognized as important, but not part of the current phase focus
* Retromount will not yet introduce a generalized input plugin system
* future input extensibility work should distinguish between source/container plugins and decoder plugins
* current built-in input handling remains in place while the presentation model is developed

## Consequences

### Positive

* keeps Phase 6 focused and achievable
* avoids mixing two separate extensibility problems in one phase
* allows the declarative presentation model to stabilize before new input abstractions are introduced
* makes later input-plugin design easier because the output side will no longer be in flux

### Negative

* input extensibility remains limited in the short term
* some desired end-to-end ecosystem support may need to wait for a later input-focused phase
* current built-in input support continues to carry responsibility for supported formats and containers

### Constraints introduced by this decision

* Phase 6 should not expand into generalized source/container or decoder plugin architecture
* discussions of input plugins during Phase 6 should be treated as future-facing design notes, not implementation scope
* later input extensibility work must model source/container and decoder concerns explicitly rather than collapsing them into one plugin type

## Notes

This decision is about sequencing, not rejection. Input extensibility remains an expected future capability of Retromount, but it will be addressed after declarative presentation specifications are established.
