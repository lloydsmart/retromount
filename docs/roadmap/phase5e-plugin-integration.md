# Phase 5E: Runtime Plugin Integration

## Status

Completed

---

## Summary

Phase 5E completes the integration of runtime encoder plugins into the Retromount pipeline.

Plugins are now:

* Discovered from a filesystem directory at runtime
* Loaded into a plugin registry
* Participating in capability resolution alongside built-in encoders
* Invoked via a subprocess protocol during materialization
* Fully integrated into `inspect`, `preview`, and `mount` flows

This phase establishes a complete end-to-end path from input content to plugin-driven output artifacts.

See also: `docs/architecture/plugins.md` for the runtime plugin model.

---

## Key Outcomes

### Runtime plugin support

* Plugins are discovered from a configurable directory (`--plugin-dir`)
* Each plugin is validated via its manifest before registration
* Invalid plugins are rejected with structured diagnostics

### Protocol-driven execution

* Plugins communicate via a JSON protocol over stdin/stdout
* The protocol defines:

  * Manifest exchange
  * Capability advertisement
  * Materialization requests and responses
* Strict validation ensures:

  * Protocol compatibility
  * Manifest correctness
  * Request integrity

### Capability resolution integration

* Plugin-provided capabilities participate in the existing resolution system
* Resolution remains:

  * Deterministic
  * Fully diagnostic
  * Priority-aware
* Built-in encoders and plugins are treated uniformly

### Subprocess runtime model

* Plugins execute out-of-process via `SubprocessEncoderPluginClient`
* Runtime is hardened against:

  * Spawn failures
  * Invalid JSON responses
  * Non-zero exit codes
  * Transient execution issues (e.g. `ETXTBSY` on Linux)

---

## Fixture Plugin

A deterministic fixture plugin is included for testing and validation:

```text
plugins/fixtures/test-inline-encoder.sh
```

This plugin:

* Advertises a high-priority disc encoding capability
* Supports multiple formats (e.g. Bin, Iso, Chd)
* Returns a fixed inline payload (`"PLUGIN"`) for all materialization requests

This enables:

* Clear verification of plugin selection
* Deterministic assertions in automated tests
* Manual validation via CLI commands

---

## Test Coverage

Phase 5E introduces a deterministic integration test:

```text
tests/plugin_integration.rs
```

This test:

* Uses real decoded and normalized content (CUE/BIN disc)
* Loads the fixture plugin from a temporary plugin directory
* Runs the full pipeline:

  * identify → decode → normalize → present → resolve → materialize
* Forces a plugin-resolved capability via a controlled presenter
* Asserts that:

  * The plugin is selected
  * The output artifact is inline-backed
  * The output bytes match the fixture payload (`"PLUGIN"`)

This provides strong guarantees that:

* Plugin discovery works
* Capability resolution selects plugin implementations correctly
* Subprocess invocation and protocol handling are functioning end-to-end

---

## CLI Usage

Plugins can be enabled via:

```bash
retromount inspect <input> --plugin-dir ./plugins
```

The same applies to:

* `preview`
* `mount`

If no plugin directory is provided, only built-in encoders are used.

---

## Guarantees Established by Phase 5E

Phase 5E proves that:

* Plugins are first-class encoder implementations
* The pipeline remains intact and deterministic with external components
* Capability resolution works uniformly across built-in and plugin encoders
* The plugin protocol is stable and enforceable
* End-to-end execution is testable and reproducible

These guarantees form the foundation for future extensibility work.

---

## Non-Goals (Deferred)

The following are intentionally out of scope for Phase 5E:

* Plugin distribution or installation mechanisms
* Version negotiation beyond exact protocol matching
* In-process or WASM plugin execution
* Streaming or partial materialization support
* Performance optimisations for large-scale plugin execution

These are candidates for future phases.

---

## Outcome

Runtime plugins are now a first-class part of the Retromount architecture.

The system supports:

* Extensible encoding via external processes
* Deterministic and diagnosable capability selection
* Safe execution boundaries between core engine and plugin code

This completes the foundation for adaptive, user-extensible output pipelines.
