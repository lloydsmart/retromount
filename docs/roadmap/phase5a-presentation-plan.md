# Phase 5A — Declarative Presentation Plan

This document defines the structure, responsibilities, and constraints of the **presentation plan**, the core output of presenter plugins in Phase 5.

---

## Goal

Introduce a **declarative representation of intended output** that:

* separates *what should exist* from *how it is materialised*
* enables host-driven capability resolution
* supports multiple encoders within a single presentation
* remains fully deterministic and inspectable

---

## Core Principle

> A presenter describes **what to produce**, not **how to produce it**.

The presentation plan is:

* **declarative** — no execution logic
* **serialisable** — transferable across process boundaries
* **encoder-agnostic** — no encoding decisions embedded
* **host-resolved** — all materialisation decisions occur in the host

---

## High-Level Structure

A presentation plan represents a **logical filesystem tree** composed of:

* directories
* files (artifacts)
* optional compound/grouped outputs

```text
PresentationPlan
└── RootDirectory
    ├── Directory
    │   └── File (ArtifactRequest)
    └── File (ArtifactRequest)
```

---

## Core Types

### PresentationPlan

Top-level container.

* `entries: Vec<PlanEntry>`
* `metadata: PlanMetadata` (optional)

---

### PlanEntry

Represents a node in the logical tree.

```rust
enum PlanEntry {
    Directory(PlanDirectory),
    File(PlanFile),
}
```

---

### PlanDirectory

* `name: String`
* `entries: Vec<PlanEntry>`

---

### PlanFile

Represents a file to be materialised.

* `name: String`
* `artifact: ArtifactRequest`

---

## ArtifactRequest

The key abstraction.

Describes **what kind of output is required**, without specifying how it is produced.

### Structure

* `id: ArtifactId`
* `source: SourceRef`
* `requirements: CapabilityRequirements`
* `metadata: ArtifactMetadata` (optional)

---

### ArtifactId

* deterministic identifier
* used for tracing, diagnostics, and caching

---

### SourceRef

Describes the input content backing the artifact.

Examples:

* ROM part
* disc image
* grouped game content
* synthetic/virtual content (e.g. playlist)

This should reference **normalized content**, not raw inputs.

---

### CapabilityRequirements

Defines what kind of encoder is needed.

Examples:

* `format: "iso" | "chd" | "zip" | "directory"`
* `multi_part: bool`
* `supports_streaming: bool`
* `content_type: "disc" | "rom" | "playlist"`

Must be:

* declarative
* matchable against encoder capabilities
* extensible

---

### ArtifactMetadata (optional)

Hints or context that do not affect correctness:

* preferred naming hints (optional — naming still policy-driven)
* grouping hints
* platform-specific metadata

---

## Compound Outputs

Some outputs are not simple 1:1 mappings from source content.

Examples:

* M3U playlists
* multi-disc groupings
* merged archives

These are represented as **ArtifactRequests with multiple SourceRefs**.

Example:

```text
ArtifactRequest (playlist)
  sources:
    - disc1
    - disc2
    - disc3
```

The presenter defines the relationship; the host resolves how to encode it.

---

## Responsibilities

### Presenter

Must:

* produce a complete and valid presentation plan
* define logical structure (directories, grouping)
* describe artifact intent via requirements
* remain deterministic

Must NOT:

* choose encoders
* perform encoding
* depend on runtime-specific behaviour
* access filesystem or external systems

---

### Host

Responsible for:

* resolving each ArtifactRequest to an encoder
* invoking encoders
* assembling final VFS structure
* handling failures and fallbacks
* providing diagnostics

---

### Encoder

Responsible for:

* declaring capabilities
* accepting materialisation requests
* producing output compatible with requirements

Encoders do not influence plan structure.

---

## Determinism

The plan must be:

* stable for identical inputs and policy
* independent of plugin load order
* free of non-deterministic identifiers

This is critical for:

* caching
* debugging
* reproducibility

---

## Diagnostics

The plan should enable the host to answer:

* what artifacts were requested
* which encoder was selected
* why a given encoder matched
* where failures occurred

Artifact IDs must be traceable through the system.

---

## Serialization

The plan must be serialisable:

* JSON (initial implementation)
* future formats possible

Requirements:

* stable schema
* versionable
* forward-compatible where possible

---

## Design Constraints

* no embedded executable logic
* no host-specific assumptions
* no encoder-specific behaviour
* minimal but expressive schema
* extensible without breaking compatibility

---

## Future Considerations

* streaming artifact support
* incremental materialisation
* caching keyed by ArtifactId
* richer capability negotiation
* plan validation tooling
* schema versioning

---

## Summary

The presentation plan introduces a **clean architectural boundary**:

* presenters define intent
* the host resolves execution
* encoders provide capabilities

This enables Retromount to evolve into a **capability-driven, runtime-extensible system** without compromising determinism or architectural clarity.
