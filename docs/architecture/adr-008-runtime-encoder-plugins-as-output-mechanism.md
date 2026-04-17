# ADR-008: Runtime Encoder Plugins as the Primary Output Materialization Mechanism

## Status

Accepted

## Context

Retromount’s output pipeline now consists of:

1. normalized content
2. presentation planning
3. capability resolution
4. artifact materialization

Phase 5 introduced runtime encoder plugins and capability-based resolution. The stabilization pass confirmed that this mechanism works through mount preparation as well as direct pipeline execution, and that meaningful failure modes are surfaced with usable diagnostics.

This establishes a clean separation:

* presentation determines **what** artifacts and structure are desired
* encoders determine **how** those artifacts are materialized

As Retromount moves toward declarative presentation specifications, it must also clarify the role of real encoder plugins in the architecture.

## Decision

Retromount will use **runtime encoder plugins as the primary extensibility mechanism for output materialization**.

Declarative presentation specifications will express artifact requirements such as content type, format, and features. Capability resolution will then select the most appropriate encoder from the available built-in and runtime plugins.

This means that:

* presentation specifications remain declarative and do not contain encoder implementation logic
* real-world output support is added primarily by implementing encoder plugins
* built-in encoders may remain for core functionality, but the architecture is centered on capability-driven resolution across both built-in and runtime encoders
* Phase 6 should include at least one real encoder plugin to validate the model against practical usage rather than fixtures alone

## Consequences

### Positive

* preserves the Presenter/Encoder separation established earlier
* keeps output materialization extensible without coupling it to presentation definitions
* allows declarative presentation specifications to remain consumer-focused
* supports deterministic resolution with diagnostics
* provides a clear path for adding real ecosystem support incrementally

### Negative

* real-world usefulness depends on building actual encoder plugins, not only fixtures
* encoder plugin quality and capability modelling become important parts of platform evolution
* some consumer support may require multiple encoder plugins over time

### Constraints introduced by this decision

* presentation specifications should request desired artifacts, not name specific implementations by default
* encoder plugins must advertise capabilities clearly enough for deterministic resolution
* runtime encoder plugins must remain compatible with the existing materialization boundary and protocol

## Notes

For Phase 6, a real ISO encoder plugin is a strong candidate because it aligns naturally with a simple target presentation such as PS2 OPL, where the desired output is a flat set of ISO images.
