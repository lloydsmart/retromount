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

**Status:** Implemented

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

The implemented boundary represents encoded input as an opaque reader handle,
known length, declared access capability, and structured source origin.
Filesystem files and stored ZIP entries advertise random access. Compressed ZIP
entries remain available to ordinary sequential-friendly content paths but are
rejected by ISO and CHD decoders, which require efficient random access.

[ADR-011](../architecture/adr-011-compressed-container-random-access.md)
defines the future opt-in, bounded disk cache that will make compressed disc
entries seekable without introducing a converted output image.

### Milestone 2 acceptance criteria

* The same decoder consumes filesystem and supported ZIP-backed content.
* Decoders no longer parse `zip:<archive>#<entry>` strings.
* Stored ISO input inside ZIP follows the same canonical DVD path.
* Existing ROM, text, CUE-relative-reference, and VFS ZIP behavior is either
  preserved or deliberately superseded with migration coverage.
* Unsupported compressed random-access inputs fail before mount preparation.

## Milestone 3: PS2 CD through OPL

**Status:** Implemented

### Milestone 3 purpose

Extend the existing consumer presentation to PS2 games distributed on CD
media.

[ADR-012](../architecture/adr-012-ps2-cd-opl-contract.md) defines cooked ISO and
CUE/BIN input, live 2352-to-2048-byte data-sector projection, preservation of
mixed-mode and audio tracks, explicit media context, the OPL-compatible output
subset, and declarative sibling `CD/` and `DVD/` presentation directories.

### Milestone 3 required spike

Confirm the exact canonical input and ISO-compatible output needed by current
OPL versions, including sector layout and any required conversion from common
PS2 CD source formats.

### Milestone 3 scope

* introduce a live track-aware CD model that preserves CUE/BIN layout, raw
  sectors, mixed-mode discs, and audio tracks;
* distinguish PS2 CD from PS1 CD and PS2 DVD;
* add an OPL `CD/<title>.iso` rule alongside the existing `DVD` rule;
* implement cooked ISO and CUE/BIN PS2 CD inputs;
* project validated MODE1/2352 and MODE2/2352 Form 1 sectors to OPL's
  2048-byte logical ISO view;
* reject layouts that OPL cannot represent without loss.

### Milestone 3 acceptance criteria

* DVD inputs continue to appear only below `DVD/`.
* Supported PS2 CD inputs appear only below `CD/`.
* PS1 CDs and unknown CDs fail closed for the OPL presentation.
* Output sector mapping and length match the confirmed OPL contract.
* No lossy normalization is used to force unsupported CD layouts into ISO.
* Mixed-mode structure, audio tracks, indexes, and pregaps survive decoding
  even when the OPL presentation rejects the disc.

## Milestone 4: First PS1 vertical slice

**Status:** Contract accepted

### Milestone 4 purpose

Introduce a track-aware optical-media use case and the first dedicated PS1
consumer presentation.

[ADR-013](../architecture/adr-013-ps1-duckstation-presentation-contract.md)
selects DuckStation as the first named consumer. The first implementation
presents a single PS1 CUE/BIN disc as a generated CUE file and live per-track
BIN files.

### Milestone 4 scope

* refine the canonical CD model to distinguish file-backed and declared
  pregaps and retain complete encoded track extents;
* add platform-specific single-disc selection without requiring CD/DVD media
  projection;
* add a multi-file CUE/BIN artifact-set contract;
* generate deterministic CUE documents with sibling per-track BIN references;
* expose every track BIN as a bounded live view over encoded sectors;
* add a serialized built-in `duckstation` presentation rooted at `PS1/`;
* prove mixed data/audio, pregap, filesystem, and stored-ZIP behavior.

### Milestone 4 delivery sequence

Milestone 4 is a group of tracked vertical increments. It is complete only
when PS1-1 through PS1-7 are implemented or a later ADR explicitly supersedes
an item.

#### PS1-1: DuckStation CUE/BIN presentation

**Status:** Implemented

Implement the first slice described above.

#### PS1-2: CHD input

**Status:** Implemented

Decode CHD metadata, tracks, sectors, audio, pregaps, and available subchannel
information into the canonical `CdDisc`. Present the result through the
DuckStation CUE/BIN view without flattening it to `LogicalDisc`.

Acceptance requires mixed-mode and audio fixtures, bounded random access, and
equivalent canonical semantics for matching CHD and CUE/BIN images.

The implementation reads CD frames live through the existing bounded CHD hunk
reader, removes CHD's 96-byte interleaved subchannel area and per-track
four-frame padding from the main track views, and preserves declared versus
file-backed pregaps. Available `RW` and `RW_RAW` subchannel bytes are retained
as separate canonical track content. Because CUE/BIN cannot carry those bytes,
the DuckStation CUE/BIN encoder fails closed for such a CHD. PS1-3 adds
source-backed SBI sidecars separately; embedded CHD subchannel conversion or a
subchannel-capable output must be specified before this case can be presented.

#### PS1-3: SBI sidecars

**Status:** Implemented

Resolve and preserve optional source-backed SBI sidecars. Couple the output
basename to the presented disc basename and reject ambiguous or mismatched
sidecars.

Acceptance requires filesystem and supported ZIP-backed sidecars, byte-exact
live reads, and a DuckStation layout in which the SBI is adjacent to its disc
entry with the same basename.

The implementation discovers one case-insensitive, same-stem `.sbi` sibling
for CUE or CHD input, validates its `SBI\0` header and complete type-1 SubQ
records, and attaches its live content to the canonical CD. Multiple matching
siblings fail as ambiguous. The DuckStation CUE/BIN artifact set emits the
sidecar adjacent to the generated CUE using the generated disc basename, and
the original SBI source is suppressed as consumed input.

#### PS1-4: Multi-disc M3U

**Status:** Implemented

Present every disc in a normalized multi-disc PS1 game and generate an M3U
whose relative entries reference the planned CUE or CHD artifacts.

Acceptance requires deterministic disc ordering, atomic conflict resolution
for the complete game artifact set, relative portable paths, and DuckStation
validation of disc switching and game grouping.

The implementation allocates one game directory before expanding its ordered
disc parts. Each disc is a nested atomic CUE/BIN artifact set, and the sibling
M3U references its generated CUE through a forward-slash relative path.
Dedicated CUE and CHD decoders share the existing `(Disc N)` filename parsing
so normalization can group and order both input formats consistently.

#### PS1-5: Cooked ISO input

**Status:** Implemented

Accept cooked ISO only for the explicit data-only PS1 subset that it can
represent. Require PS1 platform and CD media context; do not infer completeness
from the extension or synthesize absent audio, raw-sector, or subchannel data.

Acceptance requires byte-exact live reads, explicit rejection of incompatible
claims, and presentation through a consumer representation that honestly
describes the available sectors.

The DuckStation composition supplies the otherwise unknowable PS1 CD context
and decodes a cooked ISO as exactly one `MODE1/2048` data track. The source
reader backs both the canonical logical-disc view and the encoded track view,
so generated CUE/BIN output is byte-exact and does not synthesize raw-sector
headers, audio, pregaps, or subchannel data. Empty images and sizes that do not
contain whole 2048-byte sectors fail closed. Filesystem and stored-ZIP inputs
use the same live path, while OPL retains its independently selected CD/DVD
media behavior.

#### PS1-6: Native CHD output

**Status:** Planned

Add lossless CHD output for DuckStation after specifying how a compression
encoder fits a lazy, random-access VFS. The design must set bounded temporary
storage, cancellation, cache lifetime, concurrency, and repeat-mount behavior.

Acceptance requires round-trip track/layout verification, measured resource
bounds, no untracked permanent intermediate, and useful random-read behavior
after materialization.

#### PS1-7: Extend the OPL presentation with POPS

**Status:** Research required

Extend the existing `opl` presentation so a PS2 library view contains PS2
games below `DVD/` and `CD/` and PS1 games below `POPS/`. Do not create a
standalone POPS presentation.

First select the exact supported OPL, POPStarter, or POPSLoader compatibility
target and storage backend. Record the VCD format, `POPS/` layout,
launcher/runtime asset boundary, naming, multi-track, audio, and multi-disc
requirements in a dedicated ADR.

Acceptance requires:

* a documented VCD byte and sector contract derived from the selected
  implementation or its authoritative tooling;
* a declarative `POPS/` rule in the existing serialized `opl` presentation
  rather than consumer-specific decoder logic;
* preservation of the existing PS2 `DVD/` and `CD/` behavior in the expanded
  OPL view;
* preservation or explicit compatibility treatment of data and audio tracks;
* actionable handling of user-supplied launcher/runtime assets;
* a representative real-consumer test on the selected PS2 storage backend;
* documented exclusions for other forks or backends not covered by that test.

The POPS rule must reuse canonical `CdDisc` semantics but must not reuse
DuckStation's output artifact contract. VCD conversion remains a separately
resolved encoder capability even though its result is composed into the OPL
presentation.

### Milestone 4 acceptance criteria

* PS1 and PS2 CDs remain semantically distinct.
* The canonical model preserves encoded track bytes, data/audio kinds, sector
  formats, indexes, and both pregap forms required by generated CUE/BIN output.
* A mixed-mode fixture is exposed as one generated CUE and complete live track
  BINs without a whole-disc intermediate.
* Generated CUE references are relative sibling filenames and match planned
  VFS artifacts.
* Filesystem and stored-ZIP inputs produce equivalent presentation semantics.
* The complete artifact set is planned atomically and fails closed when timing
  or naming cannot be represented.
* PS2 and unknown CDs do not match the DuckStation rule.
* Multi-disc support remains deferred from this first implementation.
* PS1-6 and PS1-7 remain visible as planned work after PS1-5 merges.

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
