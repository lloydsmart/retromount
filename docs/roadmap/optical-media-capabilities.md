# Optical Media Capability Roadmap

## Goal

Expand Retromount beyond its first PS2 DVD CHD path without combining source
containers, disc formats, media semantics, platforms, and consumer
presentations into one implementation.

This roadmap implements
[ADR-010](../architecture/adr-010-optical-media-contract.md) as a sequence of
independently deliverable milestones.

## Current baseline

Retromount currently supports:

* live, bounded random-access reads from supported PS2 DVD CHDs;
* a canonical contiguous DVD representation;
* logical DVD-to-ISO materialization;
* an OPL `DVD/<title>.iso` presentation;
* reader-backed VFS files;
* older ZIP enumeration and source-backed reads for basic content.

The existing ZIP path is not the target architecture for live decoded content.
In particular, path-based decoders cannot consume ZIP entries through the same
contract as filesystem inputs.

## Guiding constraints

Every milestone must preserve these rules:

* no whole-disc intermediate is created merely to cross a pipeline boundary;
* decoders do not know consumer layouts;
* presentations do not select input formats or named encoders;
* source/container handling is independent of format decoding;
* media information is preserved when a contiguous 2048-byte view would be
  lossy;
* platform identification is explicit and is not inferred solely from a disc
  container or extension;
* support fails clearly when required random-access capabilities are absent.

## Milestone 1: Filesystem ISO input for PS2 DVD

**Status:** Implemented

### Milestone 1 purpose

Prove that multiple input decoders can produce the same canonical logical DVD
and feed the same output presentation.

### Milestone 1 scope

* identify a filesystem ISO as a distinct disc input;
* expose it as reader-backed 2048-byte logical DVD content;
* normalize it as PS2 when the OPL composition supplies the explicit platform
  context;
* reuse the logical-DVD ISO encoder;
* emit `DVD/<title>.iso` through the existing OPL specification.

### Milestone 1 non-goals

* probing arbitrary ISO contents to determine a platform;
* CD media;
* raw-sector or multi-track images;
* ISO entries inside ZIP files;
* a general input plugin API.

### Milestone 1 acceptance criteria

* CHD and ISO inputs produce equivalent canonical DVD requirements.
* Arbitrary and unaligned output reads match the source ISO bytes.
* The output length exactly matches the input logical length.
* No ISO-sized copy or cache is created.
* Invalid geometry and empty input fail with actionable errors.
* The CHD path remains unchanged.

## Milestone 2: Live input-content boundary and ZIP replacement

**Status:** Next

### Milestone 2 purpose

Allow identifiers and decoders to consume content independently of whether it
comes from the filesystem or a container.

### Milestone 2 design checkpoint

Before implementation, define:

* the resolved input-content handle and its lifetime;
* size, sequential-read, and random-read capabilities;
* how identifiers inspect content without consuming decoder state;
* error behavior for a source that cannot satisfy decoder requirements;
* ownership of nested source addressing.

### Milestone 2 scope

* centralize filesystem and ZIP `SourceRef` resolution;
* remove ZIP parsing from individual decoders and VFS consumers where the
  resolver should own it;
* preserve ordinary compressed-ROM and sidecar use cases;
* support stored ZIP entries as true random-access input;
* exercise ISO decoding through a stored ZIP entry;
* define explicit behavior for compressed random-access disc entries.

### Milestone 2 initial policy for compressed disc entries

Until a bounded strategy is designed and measured, a decoder that requires
efficient random access may reject a compressed ZIP entry with an actionable
diagnostic. It must not silently extract a complete disc, repeatedly decompress
unbounded data, or retain a complete decompressed image in memory.

### Milestone 2 acceptance criteria

* The same decoder consumes filesystem and supported ZIP-backed content.
* Decoders no longer parse `zip:<archive>#<entry>` strings.
* Stored ISO input inside ZIP follows the same canonical DVD path.
* Existing ROM, text, CUE-relative-reference, and VFS ZIP behavior is either
  preserved or deliberately superseded with migration coverage.
* Unsupported compressed random-access inputs fail before mount preparation.

## Milestone 3: PS2 CD through OPL

### Milestone 3 purpose

Extend the existing consumer presentation to PS2 games distributed on CD
media.

### Milestone 3 required spike

Confirm the exact canonical input and ISO-compatible output needed by current
OPL versions, including sector layout and any required conversion from common
PS2 CD source formats.

### Milestone 3 scope

* introduce only the CD semantics required by the confirmed OPL contract;
* distinguish PS2 CD from PS1 CD and PS2 DVD;
* add an OPL `CD/<title>.iso` rule alongside the existing `DVD` rule;
* implement one supported PS2 CD input path end to end.

### Milestone 3 acceptance criteria

* DVD inputs continue to appear only below `DVD/`.
* Supported PS2 CD inputs appear only below `CD/`.
* PS1 CDs and unknown CDs fail closed for the OPL presentation.
* Output sector mapping and length match the confirmed OPL contract.
* No lossy normalization is used to force unsupported CD layouts into ISO.

## Milestone 4: First PS1 vertical slice

### Milestone 4 purpose

Introduce a track-aware optical-media use case and the first dedicated PS1
consumer presentation.

### Milestone 4 required decisions

Choose a named consumer before fixing the output contract. The design must
then determine:

* accepted filesystem layout and artifact formats;
* treatment of data and audio tracks;
* sector formats, pregaps, indexes, and information that must be preserved;
* single-disc and multi-disc behavior;
* whether CUE, BIN, CHD, and M3U artifacts are source-backed, generated, or
  transformed.

### Milestone 4 initial implementation

Implement one input format and one consumer output end to end. Add further PS1
input formats only after the canonical CD representation is proven by that
slice.

### Milestone 4 acceptance criteria

* PS1 and PS2 CDs remain semantically distinct.
* The canonical model preserves everything required by the selected consumer.
* A mixed-mode or multi-track fixture prevents regression to a DVD-like
  contiguous-data assumption.
* Multi-disc support is included only when required by the chosen first
  consumer contract.

## Milestone 5: Performance, caching, and optimization

Begin measurement-led optimization after the preceding milestones provide
several representative workloads:

* CHD hunk decompression;
* filesystem ISO identity reads;
* ZIP-backed reads;
* PS2 DVD and CD access;
* track-aware PS1 access.

Establish benchmarks and observability before changing cache policy. Candidate
work includes:

* reader and handle reuse;
* decompressed-hunk cache tuning;
* ZIP access strategy;
* concurrent-read behavior;
* cache placement and lifetime;
* bounded memory targets.

## Later capability expansion

Later vertical slices may cover Sega CD, PC Engine CD, Atari Jaguar CD, 3DO,
CD-i, Dreamcast GDI/GD-ROM, GameCube MiniDVD, and other disc formats and
consumer views.

Their known differences are constraints on extensibility, not requirements for
the first ISO, PS2 CD, or PS1 implementation. Each new slice should extend the
canonical model only as required by a concrete decoder and presentation.
