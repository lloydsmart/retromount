# Plugins

## Overview

Retromount supports runtime-extensible output encoding through a plugin system.

Plugins allow external programs to participate in the materialization phase of the pipeline, enabling new output formats and behaviors without modifying the core codebase.

Plugins are executed out-of-process and communicate with Retromount via a structured JSON protocol over standard input/output.

---

## Goals

The plugin system is designed to:

* Enable extensibility without compromising core stability
* Keep the core model and pipeline deterministic and testable
* Allow independent development of encoders
* Support safe execution through process isolation
* Provide a clear contract between the host and plugins

---

## Non-Goals

* In-process plugins or shared library loading
* Plugin-managed presentation or structural layout
* Stateful or long-lived plugin execution
* Automatic plugin installation or distribution

---

## High-Level Architecture

Plugins integrate into the **materialization stage** of the Retromount pipeline:

```text
NormalizedContent
        ↓
    Presenter
        ↓
 PresentationPlan
        ↓
 CapabilityResolver
        ↓
 SelectedCapability
        ↓
 Encoder (built-in or plugin)
        ↓
 MaterializedArtifact
        ↓
        VFS
```

* Presenters define *what* should be produced
* Capability resolution selects *how* it should be produced
* Plugins participate as **encoders** that implement specific capabilities

---

## Plugin Lifecycle

At runtime, plugins follow a strict lifecycle:

### 1. Discovery

* Retromount scans the configured plugin directory
* All executable files are considered candidates
* Non-executable files are ignored

### 2. Manifest Retrieval

Each plugin is invoked with a `GetManifest` request:

```json
{ "type": "get_manifest" }
```

The plugin must respond with:

```json
{
  "type": "manifest",
  "manifest": { ... }
}
```

### 3. Validation

The manifest is validated to ensure:

* Required fields are present
* Capability definitions are valid
* Protocol version matches the host

Invalid plugins are rejected and excluded from the registry.

### 4. Registration

Valid plugins are registered as encoder providers.

Each declared capability becomes available to the capability resolver.

### 5. Resolution

During materialization:

* The capability resolver evaluates all available encoders
* This includes both built-in encoders and plugin-provided capabilities
* The best match is selected deterministically

### 6. Materialization

The selected plugin is invoked with a `Materialize` request:

```json
{
  "type": "materialize",
  "request": { ... }
}
```

The plugin must respond with:

```json
{
  "type": "materialized",
  "response": { ... }
}
```

---

## Execution Model

Plugins are executed as subprocesses:

* One process per request
* Communication via stdin/stdout
* No shared memory with the host
* No persistent state between invocations

### Properties

* Isolation: plugins cannot crash the host
* Determinism: each invocation is independent
* Simplicity: no lifecycle management required

---

## Protocol

Plugins communicate using JSON messages defined in the plugin protocol.

### Requests

* `get_manifest`
* `materialize`

### Responses

* `manifest`
* `materialized`
* `error`

### Versioning

Plugins must declare a protocol version:

```json
{
  "protocol_version": { "major": 1, "minor": 0 }
}
```

The host enforces strict compatibility.

---

## Capabilities

Plugins declare one or more capabilities in their manifest.

A capability defines:

* `capability_id`
* `content_type`
* supported `formats`
* supported `features`
* optional `priority`

Example:

```json
{
  "capability_id": "disc.chd",
  "content_type": "disc",
  "formats": ["chd"],
  "features": ["lossless"],
  "priority": 100
}
```

Capabilities are used by the resolver to select the best encoder for a given artifact.

---

## Artifact Types

Plugins must support one or both of:

### Source-backed artifacts

* Derived from one or more input files
* Example: converting `.bin` → `.chd`

### Generated artifacts

* Created from logical references
* Example: playlist (`.m3u`)

---

## Error Handling

Plugins may return structured errors:

```json
{
  "type": "error",
  "error": {
    "code": "some_error",
    "message": "Something went wrong"
  }
}
```

The host converts these into protocol errors and surfaces them in diagnostics.

---

## Example Plugin (Shell)

A minimal plugin:

```sh
#!/bin/sh

request="$(cat)"

case "$request" in
  *'"type":"get_manifest"'*)
    cat <<'EOF'
{
  "type": "manifest",
  "manifest": {
    "plugin_id": "example.inline",
    "plugin_version": "1.0.0",
    "protocol_version": { "major": 1, "minor": 0 },
    "capabilities": [
      {
        "capability_id": "text.inline",
        "content_type": "text",
        "formats": ["text"],
        "features": [],
        "priority": 0
      }
    ]
  }
}
EOF
    ;;
  *)
    cat <<'EOF'
{
  "type": "materialized",
  "response": {
    "inline": {
      "bytes": [72, 101, 108, 108, 111]
    }
  }
}
EOF
    ;;
esac
```

---

## Plugin Directory

Plugins are loaded from a directory specified at runtime:

```bash
retromount inspect <input> --plugin-dir ./plugins
```

* All executables in the directory are considered plugins
* Invalid plugins are skipped with diagnostics

---

## Test Fixtures

The repository includes fixture plugins under:

```text
plugins/fixtures/
```

These are used for:

* integration testing
* protocol validation
* development

They are not intended for production use.

---

## Future Work

Potential future enhancements include:

* WASM-based plugin runtime
* Plugin configuration support
* Distribution and packaging model
* Persistent plugin processes (if needed)

---

## Summary

The plugin system provides:

* A clean separation between core logic and extensible encoding
* A deterministic, testable execution model
* A safe and simple integration surface

Plugins are first-class participants in the encoding pipeline, enabling Retromount to adapt to new formats and ecosystems without modifying its core.
