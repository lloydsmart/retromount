# Presentation Serialization

## Status

Implemented for schema version 1.

## Goal

Make declarative presentations authorable as versioned YAML rather than only
as `PresentationSpec` values constructed in Rust.

## Delivered

* a versioned YAML document schema;
* strict parsing with unknown-field rejection;
* semantic validation with actionable errors;
* name-or-file presentation lookup;
* `--presentation <name|file>` for inspect and mount;
* legacy `--view` compatibility;
* `presentation: <name|file>` in configured views;
* legacy `presenter` compatibility with conflict detection;
* OPL migrated to `presentations/opl.yaml` and embedded in the binary;
* separation between the external schema and internal runtime types.

## Constraints

* Presentation files operate on normalized content.
* Presentation files cannot execute arbitrary code.
* Presentation files request output capabilities rather than named encoders.
* Unknown schema versions and fields fail closed.
* The file schema may evolve independently from the Rust representation.

## Follow-up

Before adding new target-specific Rust presentation construction:

1. extend schema version 1 only when a concrete consumer needs another generic
   rule;
2. express PS2 CD and the first PS1 presentation as YAML;
3. decide whether flat and grouped should move to separate YAML files or share
   a reusable rule mechanism;
4. introduce a user presentation search path only when name-based discovery of
   external files is required.
