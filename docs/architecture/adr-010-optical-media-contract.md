# ADR-010: Evolve Optical Media Through Capability-Led Vertical Slices

## Status

Accepted

## Context

Retromount's first live optical-media path presents a PlayStation 2 DVD CHD as
an ISO-compatible file in an OPL view. That path established several useful
boundaries:

* CHD is an input format;
* DVD is a media kind;
* PlayStation 2 is a platform;
* ISO is an output representation;
* OPL is a consumer presentation;
* decoded content can remain reader-backed through the VFS.

The next capabilities include filesystem ISO input, PlayStation 2 CDs,
PlayStation 1 discs, and a replacement for the older ZIP integration. Longer
term, Retromount is expected to support optical media used by systems such as
Sega CD, PC Engine CD, Atari Jaguar CD, 3DO, CD-i, Dreamcast, and GameCube.

Those systems cannot all be represented accurately as a contiguous stream of
2048-byte sectors. Relevant differences may include:

* CD and DVD media;
* cooked and raw sectors;
* data and audio tracks;
* multiple sessions or tracks;
* pregaps and index points;
* subchannel data;
* platform-specific metadata;
* higher-density and otherwise non-standard layouts.

Designing a universal optical-disc model before implementing those consumers
would require assumptions that have not yet been tested. Extending the current
DVD model ad hoc for each platform would instead couple input formats, media
semantics, and consumer output requirements.

The existing ZIP implementation also predates live decoded content. It embeds
container addressing in `SourceRef` strings and requires some decoders to know
whether content came from the filesystem or an archive. Path-based decoders
cannot consume nested content through the same contract as filesystem content.

## Decision

Retromount will evolve optical-media support through small, capability-led
vertical slices.

The architecture will keep the following concepts distinct:

| Concept | Responsibility | Examples |
| --- | --- | --- |
| Source/container | Locate and expose input bytes | filesystem, ZIP |
| Input format/decoder | Interpret encoded input | ISO, CHD, CUE/BIN |
| Media semantics | Describe the decoded carrier | CD, DVD, tracks, sectors |
| Platform semantics | Identify the target system | PS1, PS2 |
| Output representation | Describe requested artifacts | ISO, BIN, CUE, M3U |
| Presentation | Arrange artifacts for a consumer | OPL, future PS1 view |

No layer may infer that these concepts are interchangeable. In particular:

* an `.iso` extension does not by itself establish a platform;
* a CD is not necessarily a single 2048-byte data stream;
* CHD is not inherently PS1, PS2, CD, or DVD;
* ZIP is a container, not a decoder for the entries it contains;
* a presentation requests consumer-visible artifacts without selecting an
  input container.

### Canonical media model

The current `LogicalDisc` contract remains valid for media with one canonical,
contiguous logical data view. It will not be expanded immediately into a
universal optical-disc model.

Track-aware CD concepts will be introduced only when the PS2 CD and PS1 slices
define concrete requirements. Any extension must preserve information required
for correct output and must not force all media into a lossy 2048-byte-sector
view.

Future formats must be able to add richer media descriptions without changing
presentation or decoder responsibilities.

### Live input content

Decoders should consume an opaque input-content capability rather than opening
filesystem paths directly. A later source-boundary slice will define the exact
API, conceptually:

```text
SourceRef
    ↓
Source/container resolver
    ↓
Live input content / reader capability
    ↓
Identifier and decoder
```

The resolver owns filesystem and container addressing. Format decoders own
format interpretation. A decoder must not parse ZIP addressing or require a
temporary extracted path.

Not every container entry can promise efficient random access. The source
contract must expose the capabilities needed for a decoder to accept or reject
an input deliberately. Support for compressed disc images inside ZIP files
must not silently introduce whole-image extraction or unbounded memory use.

### Implementation sequence

Capabilities will be delivered in the following order:

1. filesystem ISO to canonical PS2 DVD to the existing OPL presentation;
2. a live input-content boundary and replacement ZIP integration;
3. PS2 CD support in the OPL presentation;
4. a named PS1 consumer presentation and its first input path;
5. measurement-led caching and performance work.

Each item is a separate milestone with its own acceptance criteria. Later input
formats and presentations follow the same vertical-slice approach.

## Consequences

### Positive

* The next implementation is small enough to validate decoder interchangeability.
* Sony-specific assumptions do not become universal optical-media rules.
* ZIP can be corrected at the source/container boundary instead of being
  special-cased by every decoder.
* The CD model will be driven by actual PS2 and PS1 requirements.
* Performance work will be informed by multiple realistic access patterns.
* Future platforms can extend media semantics without inheriting ISO- or
  PlayStation-specific coupling.

### Negative

* The full optical-media model remains intentionally incomplete.
* Some formats will require a design increment before implementation.
* A source capability model adds an abstraction between discovery and decoding.
* Compressed archive entries may need to be rejected for some random-access
  formats until a bounded strategy exists.

## Alternatives considered

### Design a universal optical-disc model now

Rejected. The known future systems demonstrate the need for extensibility, but
do not yet provide sufficiently concrete output contracts to validate one
complete model.

### Extend `LogicalDisc` separately for each new platform

Rejected. This would encode platform and input-format assumptions in the
canonical media layer and make cross-format reuse difficult.

### Begin with caching and performance

Deferred. The current CHD reader already has bounded hunk caching, and there is
not yet enough diversity of readers and media workloads to identify the right
system-wide cache boundaries.

### Design input plugins before adding more built-in decoders

Deferred. ISO and the revised ZIP path should first provide additional evidence
about the source/container and decoder contracts. Plugin transport can then be
designed around demonstrated capabilities.

## Non-goals

This decision does not define or implement:

* a complete track/session/subchannel model;
* PS1 output layout;
* PS2 CD conversion rules;
* compressed random access inside ZIP files;
* Dreamcast GDI/GD-ROM or GameCube MiniDVD semantics;
* external input plugins;
* new caching policies.

These are follow-on decisions or implementation milestones.
