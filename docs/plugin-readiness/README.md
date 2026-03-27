# Plugin Readiness Review

This document set tracks the work required to prepare Retromount for a future
plugin-based architecture.

## Goals

The goal of this review is to:

- Ensure core pipeline stages are extensible via well-defined trait boundaries
- Introduce a clean composition boundary for pipeline components
- Enable future support for:
  - Alternate presenters (e.g. MiSTer, Batocera, PS2)
  - Alternate encoders
  - Additional input/decoder implementations
- Lay the groundwork for eventual runtime plugin loading (.so/.dll)

## Scope

This phase focuses on compile-time composition only.

We are **not** introducing:

- Dynamic plugin loading
- A plugin registry system
- Pluggable normalization

Instead, we aim to:

- Centralize construction of pipeline components
- Remove hardcoded dependencies from application entrypoints
- Ensure the engine operates purely on trait-based abstractions

## Structure

- `review-findings.md` — architectural issues identified
- `decisions.md` — agreed design decisions
- `cleanup-tracker.md` — concrete implementation tasks

## Relationship to Architecture Review

This work builds on the architecture boundary review:

- Pipeline stages (decode → normalize → present) are already well-defined
- Presenter/encoder responsibilities are separated
- Core models are presentation-agnostic

The plugin-readiness review focuses on how implementations are composed and injected.
