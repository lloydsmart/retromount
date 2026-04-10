# Phase 5C — Encoder Plugin Protocol

This document defines the protocol boundary between the Retromount host and externally provided encoder plugins.

Phase 5A introduced the declarative presentation plan.

Phase 5B introduced deterministic capability resolution against encoder-declared capabilities.

Phase 5C builds on those foundations by defining the **stable plugin-facing contract** that allows encoder implementations to participate in the runtime without being compiled directly into the main binary.

---

## Goal

Define a **stable, serialisable, versioned protocol** for encoder plugins that allows the host to:

* discover plugin metadata
* inspect declared encoder capabilities
* select a capability using the host resolution engine
* request materialisation of a planned artifact
* receive a structured materialisation result
* surface protocol and runtime failures cleanly

Specifically:

> Encoder plugins should be replaceable runtime participants that declare capabilities and materialise artifacts through a host-controlled protocol boundary.

---

## Core Principle

> The host owns planning and resolution. The plugin only declares capabilities and materialises artifacts requested by the host.

The protocol must preserve the architectural boundaries established in earlier phases:

* **presenters define what should exist**
* **the host resolves which capability should satisfy each request**
* **plugins materialise artifacts only after selection**
* **plugins do not perform negotiation or planner-side decision-making**

---

## Scope

Phase 5C includes:

* protocol-level plugin metadata
* protocol-level capability advertisement
* protocol request and response types for encoder materialisation
* protocol error model
* protocol versioning and compatibility rules
* validation requirements for protocol payloads

Phase 5C does **not** include:

* dynamic loading
* process management
* transport details such as pipes, sockets, or RPC framework selection
* plugin discovery paths
* sandboxing or security isolation
* implementation of a real external plugin

Those concerns belong to later phases.

---

## Why a Protocol Boundary Exists

Built-in encoders can implement internal Rust traits directly.

External plugins cannot safely depend on internal host types as their compatibility boundary.

A protocol boundary is required so that:

* compatibility can be versioned explicitly
* plugin and host can evolve independently
* transport/runtime model remains replaceable
* future plugins are not forced to be Rust crates linked against host internals

This means the plugin-facing model should be expressed as **serialisable protocol data**, not as direct exposure of internal engine types.

---

## Architectural Position

The protocol sits between **host-side resolution/materialisation orchestration** and **plugin-side encoder implementation**.

```text
NormalizedContent
    ↓
Presenter
    ↓
PresentationPlan
    ↓
Host Capability Resolution
    ↓
Selected Capability
    ↓
Plugin Protocol Request
    ↓
Encoder Plugin
    ↓
Plugin Protocol Response
    ↓
Host VFS Materialisation
```

---

## Protocol Design Principles

### Host-driven selection

The plugin advertises capabilities, but does not choose which one is used.

The host always sends the selected capability ID as part of a materialisation request.

---

### Declarative capability advertisement

Capabilities must be described as plain structured data.

No imperative “can you handle this?” negotiation is allowed in protocol v1.

---

### Stable serialisable types

All protocol messages must be serialisable and independent of internal Rust trait objects or engine-only abstractions.

---

### Explicit compatibility

Protocol compatibility must be stated explicitly via version metadata.

No implicit compatibility assumptions are allowed.

---

### Diagnostics-friendly failures

Errors must be structured so that the host can distinguish:

* protocol incompatibility
* invalid request payloads
* unsupported capability selection
* plugin execution failure

---

## Protocol Surface

Phase 5C defines four protocol areas:

1. plugin metadata
2. capability advertisement
3. materialisation request/response
4. protocol errors

---

## 1. Plugin Metadata

A plugin must expose metadata describing its identity and compatibility.

### PluginManifest

A plugin manifest should include:

* `plugin_id`
* `plugin_version`
* `protocol_version`
* `display_name` (optional)
* `description` (optional)
* `capabilities`

This metadata is returned before any materialisation call occurs.

### Responsibilities

The manifest allows the host to:

* identify the plugin deterministically
* validate protocol compatibility
* inspect available capabilities
* expose plugin information to users

---

## 2. Capability Advertisement

Capabilities are the plugin-facing equivalent of the host’s internal encoder capability model.

### ProtocolEncoderCapability

Each capability should include:

* `capability_id`
* `content_type`
* `formats`
* `features`
* `priority`
* `supports_multi_source`

This model should map cleanly onto host-side resolution data, while remaining protocol-stable.

### Capability Semantics

A capability describes what the plugin can materialise.

It must not depend on dynamic host negotiation.

The host should be able to evaluate capability suitability without calling plugin code.

---

## 3. Materialisation Request/Response

Once the host selects a capability, it sends a materialisation request to the plugin.

### MaterializationRequest

The request should include:

* `artifact_id`
* `logical_name`
* `selected_capability_id`
* `artifact_kind`
* `requirements`
* `context`

This request is intentionally richer than a bare capability ID because the plugin must know:

* what artifact is being requested
* which declared capability the host selected
* what source or generated artifact shape it is materialising
* what supporting context is available

### Artifact Kind

The request should include a protocol representation of the planned artifact kind.

At minimum this must support:

* source-backed artifacts
* generated artifacts such as playlists

The protocol representation must remain declarative.

The host sends the resolved artifact description; the plugin materialises it.

### MaterializationContext

The host may provide supporting context needed for deterministic generation.

Initial context should include:

* referenced artifact names

This allows plugins to generate outputs like playlists without host-specific hidden knowledge.

---

### MaterializationResponse

The plugin returns a materialisation result that the host can convert into a `VfsFile`.

Protocol v1 should support:

* **source-backed output**

  * reference to one source object
* **inline output**

  * complete generated bytes
* optional metadata such as:

  * size
  * media/content type (future-friendly but optional)

This keeps the protocol aligned with the current VFS materialisation model without overcommitting to future backing types.

---

## 4. Error Model

Errors must be structured and machine-readable.

### ProtocolError

The protocol should distinguish between:

* `IncompatibleProtocolVersion`
* `UnknownCapabilityId`
* `InvalidRequest`
* `MissingContext`
* `UnsupportedArtifactKind`
* `MaterializationFailed`
* `InternalPluginError`

The host may choose to surface these differently in inspect/debug views, logs, or user-facing errors.

### Error Boundaries

Errors should indicate whether the failure is caused by:

* host/plugin version mismatch
* host request construction bug
* plugin declaration inconsistency
* plugin runtime failure

This distinction is important for later runtime orchestration.

---

## Protocol Versioning

Protocol versioning must be explicit in v1.

### Requirements

* the host declares the protocol version it supports
* the plugin declares the protocol version it implements
* incompatible versions fail before capability use or materialisation

### Initial Rule

For the initial implementation, compatibility should require an exact protocol version match.

This is intentionally strict and can be relaxed later if needed.

---

## Validation Rules

Protocol payloads must be validated at the boundary.

### Manifest validation

The host should reject manifests with:

* empty plugin IDs
* duplicate capability IDs within one plugin
* invalid or missing protocol version
* malformed capability definitions

### Request validation

The plugin should reject requests with:

* unknown selected capability ID
* malformed artifact definitions
* missing required context
* unsupported artifact kinds for the selected capability

---

## Relationship to Internal Types

The protocol is expected to mirror existing internal concepts, but it must not expose internal implementation details as the stability boundary.

This means the host will likely maintain:

* internal planning and resolution types
* protocol DTOs used for plugin communication
* conversion logic between the two

That separation is intentional.

It prevents protocol compatibility from becoming coupled to internal refactors.

---

## Suggested Initial Protocol Model

Conceptually, the protocol needs types equivalent to:

* `PluginManifest`
* `ProtocolEncoderCapability`
* `ProtocolCapabilityRequirements`
* `ProtocolArtifactKind`
* `ProtocolMaterializationContext`
* `MaterializationRequest`
* `MaterializationResponse`
* `ProtocolError`

These may be represented as Rust structs/enums internally, but their real role is as the schema of the plugin boundary.

---

## Success Criteria

Phase 5C is complete when:

* a plugin-facing encoder protocol is documented
* protocol request/response and manifest types are defined in code
* protocol version compatibility rules are explicit
* protocol payload validation rules are defined
* the protocol can represent current built-in encoder use cases without leaking internal host-only abstractions

---

## Non-Goals Reaffirmed

Phase 5C does not yet prove runtime plugin loading.

It defines the contract that runtime loading will later use.

That separation is intentional:

* Phase 5C defines **what the boundary is**
* Phase 5D will define **how the host crosses it**

---

## Follow-On Work

Phase 5D will build on this by introducing:

* runtime plugin loading model
* transport/lifecycle management
* host-side plugin client implementation
* error propagation across the runtime boundary
* plugin inspection/discovery commands

A later phase can then prove the model with a real encoder plugin such as CHD.
