# Phase 5B — Capability Model and Resolution

This document defines how the host resolves `ArtifactRequest`s from the presentation plan to concrete encoder implementations.

---

## Goal

Define a **deterministic, inspectable, and extensible mechanism** for:

* describing encoder capabilities
* expressing artifact requirements
* matching requirements to capabilities
* selecting a single encoder for each artifact

---

## Core Principle

> The host selects the encoder. Plugins only declare capabilities.

Resolution must be:

* **deterministic** — identical inputs always yield the same result
* **declarative** — no imperative negotiation between host and plugins
* **inspectable** — decisions can be explained and traced
* **extensible** — new capabilities can be added without breaking compatibility

---

## Overview

Resolution operates on:

* `ArtifactRequest` (from presenter)
* `EncoderCapability` (from encoder plugins)

The host performs:

1. candidate discovery
2. capability matching
3. scoring and filtering
4. deterministic selection
5. diagnostics capture

---

## Capability Model

Encoders advertise capabilities as structured, declarative data.

### EncoderCapability

Represents a single supported capability.

* `id: CapabilityId`
* `content_type: ContentType`
* `formats: Vec<Format>`
* `features: CapabilityFeatures`
* `priority: Option<u32>`

---

### ContentType

Defines the type of input content supported.

Examples:

* `rom`
* `disc`
* `playlist`
* `archive`
* `directory`

---

### Format

Defines output format(s) supported.

Examples:

* `iso`
* `chd`
* `zip`
* `m3u`
* `directory`

Encoders may support multiple formats.

---

### CapabilityFeatures

Optional feature flags describing behaviour:

* `multi_source: bool`
* `streaming: bool`
* `lossless: bool`
* `random_access: bool`
* `supports_partial: bool`

Features must be:

* additive
* independently matchable
* safe to ignore if unknown

---

## Requirement Model

Derived from `ArtifactRequest.requirements`.

### CapabilityRequirements

* `content_type: ContentType`
* `format: Option<Format>`
* `required_features: Vec<Feature>`
* `preferred_features: Vec<Feature>`
* `forbidden_features: Vec<Feature>`

---

### Requirement Semantics

* **required_features**

  * must be present in capability
* **preferred_features**

  * improve match score but are not required
* **forbidden_features**

  * disqualify a capability

---

## Matching Rules

A capability is considered a **valid match** if:

1. `content_type` matches exactly
2. requested `format` is supported (if specified)
3. all `required_features` are present
4. no `forbidden_features` are present

Capabilities that fail any rule are rejected.

---

## Resolution Algorithm

### Step 1 — Candidate discovery

Collect all encoder capabilities across all plugins.

---

### Step 2 — Filtering

Filter capabilities to those that satisfy all matching rules.

If no candidates remain → **resolution failure**

---

### Step 3 — Scoring

Each candidate is scored based on:

* number of matched `preferred_features`
* capability `priority` (if provided)
* specificity (e.g. fewer optional features, more exact matches)

---

### Step 4 — Deterministic selection

Sort candidates by:

1. highest score
2. highest priority
3. most specific match
4. stable tie-breaker (e.g. plugin ID + capability ID)

Select the first candidate.

---

### Step 5 — Result

Return:

* selected encoder capability
* diagnostic information (see below)

---

## Failure Handling

### No matching encoder

The host must:

* fail the artifact
* provide diagnostic output explaining why no match was found

No implicit fallback is allowed in v1.

---

### Multiple equal matches

Resolved via deterministic ordering (tie-breaker).

---

### Runtime failure during materialisation

If an encoder fails after selection:

* mark artifact as failed
* optionally allow retry with next-best candidate (future consideration)

---

## Diagnostics

Resolution must produce structured diagnostics:

* list of all candidate capabilities
* rejection reasons per candidate
* scoring breakdown
* selected capability and reason

This enables:

* `inspect` output
* debugging plugin behaviour
* validating capability design

---

## User Influence (Future)

The model allows future extensions:

* user-defined encoder preference
* capability weighting overrides
* plugin enable/disable controls
* per-format or per-platform overrides

These should integrate without changing core resolution logic.

---

## Determinism Guarantees

Resolution must be:

* independent of plugin load order
* independent of runtime timing
* stable across runs given identical inputs

Tie-breaking must always produce the same result.

---

## Design Constraints

* no runtime negotiation between plugins
* no hidden fallback logic
* no implicit prioritisation outside defined rules
* minimal and explicit matching model

---

## Future Considerations

* multi-stage encoding pipelines
* composite capabilities (chained encoders)
* streaming-first resolution paths
* caching of resolution results
* capability versioning and compatibility negotiation

---

## Summary

The capability model and resolution system ensures that:

* presenters describe intent
* encoders declare capabilities
* the host performs deterministic selection

This preserves architectural boundaries while enabling flexible, extensible encoding workflows.
