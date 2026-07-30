# Phase 6 Integration Summary

Phase 6 replaced built-in Rust presenter implementations with declarative
presentation specifications compiled into `PresentationPlan`.

## Delivered

* `PresentationSpec` models layout, selection, naming, and artifact requirements.
* The generic presentation compiler supports flat and grouped layouts.
* Flat and grouped specifications cover ROMs, discs, bytes, text, and games
  without parts.
* Multi-disc games emit ordered disc artifacts and generated M3U playlists.
* Grouped layouts preserve relative paths and merge shared directories.
* Naming and conflict policies apply consistently to files and directories.
* Preview, inspect, mount, configured views, and runtime encoder plugins execute
  through spec-native pipeline entry points.
* Legacy presenters, their registry, and presenter-based pipeline paths have
  been removed.

## Validation

The integration audit confirmed:

* flat CLI inspection places nested input files at the root;
* grouped CLI inspection preserves nested relative paths;
* encoder plugin materialization remains covered end to end;
* formatting, linting, unit tests, integration tests, and documentation checks
  pass.

## Result

Adding a new consumer layout now means defining a presentation specification
and, when needed, supplying an encoder capability. It no longer requires a new
filesystem presenter implementation.

The first practical consumer target was subsequently delivered: supported PS2
DVD CHD content can be presented through a live, ISO-compatible OPL view.

The next capability roadmap is documented in
[`optical-media-capabilities.md`](optical-media-capabilities.md).
