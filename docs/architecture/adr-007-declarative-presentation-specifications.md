# ADR-007: Declarative Presentation Specifications

## Status

Accepted

## Context

Retromount now has a stable output-side architecture built around:

1. normalized content as the canonical input to presentation
2. `PresentationPlan` as the declarative description of intended output
3. capability-based encoder resolution
4. materialization through built-in and runtime encoder plugins

Phase 5 and the subsequent stabilization pass confirmed that runtime encoder plugins are viable in real execution paths, including mount preparation and failure handling.

The next architectural question is how presentation itself should be extended.

One possible direction is to implement presentation as code-only presenter plugins. Another is to define presentation declaratively and interpret those declarations into `PresentationPlan`.

Review of target output ecosystems such as MiSTer, Batocera, PS2 OPL, RetroArch, Analogue Pocket, PlusCart, and others suggests that most desired output views are primarily defined by:

* structure and grouping
* naming rules
* artifact requirements
* conditional emission rules
* conflict handling

These are better described as desired output shape than as arbitrary procedural logic.

At the same time, some future targets may require richer rules than a minimal first declarative model can express.

## Decision

Retromount will adopt a **declarative presentation model** as the primary way to define output views.

Presentation specifications will describe desired output shape over normalized content and will be interpreted into `PresentationPlan`.

This means that:

* presentation is treated as a planning problem, not primarily as arbitrary code
* declarative presentation specifications become the main extension model for views
* the declarative model may be enriched over time when real target views require additional expressiveness
* presentation specifications describe output requirements against normalized content, not against original input formats

The concrete serialization format for presentation specifications may be YAML, but the architectural decision is about the **declarative model**, not YAML specifically.

## Consequences

### Positive

* aligns presentation with the existing `PresentationPlan` boundary
* keeps output intent explicit and inspectable
* makes views easier to define, review, share, and test
* allows presentation logic to remain independent from output materialization
* matches the dominant shape of expected consumer views
* supports gradual enrichment of the model based on real requirements

### Negative

* Phase 6 must define and validate a declarative presentation model before broad implementation can proceed
* a first version of the model may not immediately express every future target
* care is required to avoid turning the declarative model into an ad hoc programming language

### Constraints introduced by this decision

* presentation declarations must operate over normalized content
* presentation declarations should not encode input-format-specific assumptions
* when the model is insufficient, it should be enriched deliberately based on concrete use cases rather than speculative design

## Notes

This decision does not require all desired views to be implemented immediately. Phase 6 will begin by expressing simple existing and target views declaratively, then expand the model only where real consumer requirements demand it.
