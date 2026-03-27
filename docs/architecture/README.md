# Architecture Docs

This directory captures the current architectural shape of Retromount following Phase 3 consolidation and the pipeline orchestration unification (D-002).

It documents the system as it exists today, along with the review work carried out in `feature/architecture-boundary-review`.  

---

## How to use these documents

The architecture documentation is structured to provide a clear, traceable view of how Retromount evolves over time.

The documents work together as follows:

- **Review findings (`review-findings.md`)**  
  Identify architectural issues, ambiguities, or areas for improvement.

- **Decisions (`decisions.md`)**  
  Record the chosen approach for addressing findings, including context and consequences.

- **GitHub issues**  
  Capture discussion, exploration, and implementation tracking for each finding.

- **Commits**  
  Implement the decisions in the codebase.

This creates a traceable flow:

```text
Finding → Decision → Issue → Implementation
```

Or when navigating in reverse:

```text
Code → Commit → Issue → Decision → Finding
```

### How to navigate

- Start with **review findings** to understand current concerns
- Follow links to **decisions** to see how those concerns were resolved
- Use linked **issues** for detailed discussion and context
- Refer to **commits** for the actual implementation

### Why this exists

This structure ensures that:

- architectural intent is preserved over time
- decisions remain understandable after implementation
- future changes can be made with confidence
- contributors can quickly understand why the system is designed the way it is  

---

## Documents

- `boundaries.md` — defines the canonical architecture, pipeline stages, and stage contracts
- `review-findings.md` — findings discovered during architecture review
- `decisions.md` — architectural decisions made during the review
- `cleanup-tracker.md` — concrete cleanup and refactor tasks arising from findings

---

## Current Architecture Overview

Retromount is built around a single, unified processing pipeline:

Input → Identify → Decode (`DecodedContent`) → Normalize (`NormalizedContent`) → Present → Encode → VFS

All execution modes (preview, inspect, and runtime) use this pipeline.

There is no alternative orchestration path.

---

## Key Concepts

### Pipeline-first design

The pipeline is the core abstraction of the system. Each stage has a clearly defined responsibility and communicates through well-defined data structures.

---

### Input sources

Content is ingested via `InputSource` implementations, which are responsible for:

- enumerating files and archive contents
- abstracting over container formats (filesystem, ZIP, etc.)
- producing a stream of input items

This replaces the earlier loader-based discovery model.

---

### Identification and decoding

Input items are:

1. identified (what might this be?)
2. decoded (what is it structurally?)

These steps transform raw input into structured data suitable for normalization.

---

### Decoded and normalized models

Retromount separates decoded content from normalized content.

Key types include:

- `DecodedContent`
- `NormalizedContent`
- `GameContent`
- `GamePart`

This split ensures that decoded artifacts and normalized semantic entities are not represented by the same top-level model.

---

### Presentation and encoding

The output stage transforms normalized content into a virtual filesystem representation.

Within that stage:

- the presenter determines structure, grouping, and layout
- the encoder materializes individual output files

In the default implementation, the presenter composes an encoder while constructing the VFS tree.

Together, they produce the final VFS tree exposed to consumers.

Key components:

- `OutputPresenter`
- `GenericPresenter`
- `BasicEncoder`
- `VfsDirectory`
- `VfsFile`

This layer defines how the content is presented to consumers.

---

## Design Goals

- Single orchestration model  
  All processing flows through the pipeline

- Clear separation of concerns  
  Each stage has a distinct and well-defined responsibility

- Presentation-independent core model  
  Internal representations are not tied to output formats

- Extensibility  
  New input sources, decoders, and output formats can be added without breaking existing stages

---

## Historical Context

Earlier versions of Retromount used a loader-based architecture involving:

- Loader
- InputRegistry
- InputHandler
- VirtualFile
- GameImage

This model introduced a second orchestration path and blurred architectural boundaries.

As part of D-002, this system has been removed in favour of the unified pipeline architecture described above.

---

## Review Goals

The architecture review work in this directory aims to:

- clarify layer boundaries
- identify confusing or redundant structures
- remove obsolete or transitional code
- ensure the system is aligned with a pipeline-first design
- prepare the codebase for future extensibility (including plugin-style architecture)

---

## Next Focus Areas

- review the next architectural boundary concern after D-005
- improve module-level documentation
- introduce diagrams for better visualization of the pipeline
