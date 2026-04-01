# Canonical External Dataset Presentation

> ⚠️ This is a forward-looking design note / aspiration.  
> It is not committed to the roadmap and has no active implementation plan.

---

## Summary

Retromount may eventually support presenting content in **externally defined canonical formats**, where the virtual filesystem exactly matches the structure, naming, and byte-level representation expected by a third-party system.

One motivating example is presenting a ROM library in a format compatible with community preservation projects distributed via BitTorrent, allowing a local collection to be **live-seeded without re-materializing data on disk**.

---

## Problem Statement

Retromount currently focuses on:

- emulator-friendly presentation
- flexible structure and naming
- transformation and normalization for usability

However, some external systems require:

- strict canonical structure
- exact filenames and layout
- byte-for-byte identical file contents

These systems are not tolerant of approximation or “equivalent” representations.

This introduces a new class of presentation target:

> A **canonical dataset view**, where Retromount must exactly reproduce an externally defined dataset contract.

---

## Key Requirements

A canonical external dataset presentation must guarantee:

### Deterministic Output

- identical inputs always produce identical presented files
- no dependence on runtime ordering, environment, or timing

### Stable File Identity

- filenames, paths, and directory structure must match the external specification exactly
- no normalization or variation unless explicitly defined by the canonical format

### Byte-Exact Representation

- file contents must match canonical payloads bit-for-bit
- no lossy or convenience transformations

### Random-Access Readability

- files must support efficient `read(offset, length)` semantics
- required for systems such as torrent clients performing piece verification

### Immutability Expectations

- presented data must remain stable over time for a given configuration
- encoder or policy changes must not silently alter outputs

---

## Architectural Fit

This concept aligns with Retromount’s existing pipeline and boundaries.

### Presenter

A canonical presenter (e.g. `MinervaPresenter`) would:

- define the exact directory structure required by the external dataset
- group and filter normalized content into canonical sets
- exclude any content that cannot be represented canonically

It answers:

> “What files and directories should exist?”

---

### Encoder

A canonical encoder (e.g. `MinervaEncoder`) would:

- enforce exact filenames and extensions
- produce byte-exact file contents
- reject any transformations that are not strictly deterministic and lossless
- guarantee stable and reproducible output

It answers:

> “What are the exact bytes of each file?”

---

### Strictness Model

Unlike emulator-oriented presentation, canonical dataset views must:

- reject unsupported content
- avoid “close enough” representations
- prioritise correctness over completeness

---

## Additional Considerations

### Canonical Identity

A mapping must exist between:

- normalized Retromount content
- canonical dataset entries

This likely requires:

- hash-based identification
- external manifests or DAT-style metadata
- strict matching rules

---

### Backing Semantics

Canonical outputs may require more advanced backing than:

- source-backed files
- inline-generated files

Some outputs may require:

- deterministic transformation with efficient random-access reads
- stable length and byte layout independent of runtime conditions

---

### Performance Constraints

Some consumers (e.g. torrent clients) may:

- read small ranges repeatedly
- access files non-sequentially
- verify data aggressively

Encoders must handle this efficiently without recomputing entire outputs.

---

## Challenges

- ensuring byte-for-byte equivalence with external datasets
- supporting efficient random-access reads for transformed content
- maintaining determinism across versions and environments
- defining and enforcing canonical mapping rules
- avoiding performance issues under heavy read patterns

---

## Non-Goals (Initial)

- full BitTorrent integration
- torrent creation or tracker interaction
- guaranteeing compatibility with all external dataset formats

The focus is on **presentation correctness**, not distribution tooling.

---

## Status

Deferred (Post Phase 5/6)

This concept is intentionally out of scope for current development phases. It represents a long-term architectural aspiration.

---

## Rationale

This idea is valuable even if never implemented directly:

- it stress-tests architectural decisions
- it highlights gaps in determinism and canonical mapping
- it aligns Retromount with broader preservation and distribution ecosystems

It represents a shift from:

> “Present content in a useful way”

to:

> “Present content in an exact, externally defined way”

---

## Future Exploration

If pursued, a sensible path would be:

1. Define a canonical dataset contract (structure, naming, hashing)
2. Build a verification mode (what content matches canonically?)
3. Implement a strict Presenter/Encoder pair
4. Evaluate feasibility of live-mounted seeding vs export-based workflows

---
