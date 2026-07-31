# ADR-013: Present PlayStation CDs to DuckStation Without Flattening Tracks

## Status

Accepted

PS1-1 through PS1-5 and PS1-6a implemented

## Context

The PlayStation 2 CD milestone introduced a live, track-aware `CdDisc` model
and CUE/BIN decoding. Its OPL presentation deliberately accepts only discs
that can be exposed as one logical 2048-byte ISO. That is not a suitable
default for PlayStation 1 media.

PlayStation discs commonly use raw 2352-byte sectors, mixed data and CD-DA
tracks, pregaps, and multiple source files. A PS1 presentation must retain
those semantics rather than reuse OPL's lossy compatibility boundary.

The first presentation needs a named consumer. DuckStation is selected because
its documented inputs include BIN/CUE and MAME CHD, it supports multi-disc
game-list grouping, and it can use adjacent SBI subchannel sidecars for
affected PAL titles. This makes it a useful concrete contract without making
its full input catalog the scope of the first implementation.

Primary references:

* [DuckStation README and supported disc formats][duckstation-readme]
* [DuckStation LibCrypt and SBI guidance][duckstation-sbi]

The current presentation model assumes that one selected item requests one
output artifact. A faithful CUE/BIN disc instead produces a related artifact
set: one generated CUE file and one or more reader-backed BIN files. Their
names and CUE references must be planned together.

The current `CdTrack` model also combines declared `PREGAP` and file-backed
`INDEX 00` duration into one count, while its data range begins at `INDEX 01`.
That is enough to reject lossy OPL projections, but not enough to reproduce a
faithful CUE/BIN layout. PS1 output requires those two pregap forms and their
source extents to remain distinguishable.

## Decision

Add a built-in `duckstation` presentation whose first vertical slice exposes
single-disc PS1 CUE/BIN content as a lossless CUE/BIN artifact set.

The first implementation will consume the existing CUE/BIN decoder. It will
not add CHD or ISO input in the same slice.

### Consumer filesystem contract

For each supported single-disc PS1 game, the presentation will request:

```text
PS1/<game>/<disc>.cue
PS1/<game>/<disc> (Track 01).bin
PS1/<game>/<disc> (Track 02).bin
...
```

`<game>` uses the policy-derived game name. `<disc>` uses the part name so the
layout can later accommodate multiple discs without renaming the first-slice
artifacts. Track numbers are zero-padded canonical track numbers.

The CUE file and every BIN it references must be siblings. Generated CUE paths
must contain filenames only, use quoted names, and must not contain host paths,
container addresses, absolute paths, or parent traversal.

The dedicated `PS1/` root is presentation data, not decoder behavior.
DuckStation can scan the root recursively; Retromount does not write into a
DuckStation configuration directory.

### Selection contract

The presentation matches only normalized `Platform::Ps1` disc games backed by
a `CdDisc`.

The first slice accepts exactly one disc per normalized game. PS2 and unknown
CDs do not match. A PS1 `LogicalDisc` alone is insufficient because it cannot
represent audio tracks or the encoded raw-sector layout.

Platform identity remains explicit composition or normalization context. CUE,
BIN, ISO, and CHD extensions do not establish PS1 identity.

### CUE/BIN artifact-set contract

One selected disc produces one atomic artifact set:

* a small generated UTF-8 CUE document;
* one reader-backed BIN artifact per canonical track;
* internal references from the CUE document to those planned BIN names.

The plan must resolve names and conflicts for the complete set before adding
any member to the VFS. A partial set must never be exposed.

This requires an explicit multi-file or artifact-set output contract.
Presentation YAML declares the desired CUE/BIN representation; it does not
list a variable number of tracks or embed CUE generation logic. The encoder
expands the selected `CdDisc` into the related files.

Capability resolution remains representation-based. The presentation requests
a lossless track-aware CUE/BIN disc artifact set, not a named Rust encoder.

### Track BIN contract

Each track BIN exposes the encoded sectors for that track without converting
data sectors to 2048-byte logical sectors and without decoding audio.

The output preserves:

* track number and order;
* data versus audio kind;
* `MODE1/2048`, `MODE1/2352`, `MODE2/2352`, or `AUDIO`;
* every sector byte represented by the canonical track extent;
* file-backed `INDEX 00` content;
* declared synthetic pregaps;
* `INDEX 01` and any other supported index positions.

Each BIN is a bounded, random-access view over its source content. Tracks that
shared one input BIN may be presented as separate live ranges; no whole-disc
or whole-track copy is required.

Splitting tracks into separate BIN files gives every generated CUE file a
uniform layout and avoids reconstructing the original source-file grouping.
This is lossless because CUE file grouping is a container arrangement, while
track sectors and timing are the media semantics that must be preserved.

### CUE generation contract

The generated CUE contains one `FILE ... BINARY` section per output track,
followed by its `TRACK` and timing directives.

For a track with file-backed pregap content, the BIN begins at that track's
`INDEX 00` extent and the CUE expresses:

```text
INDEX 00 00:00:00
INDEX 01 <file-backed-pregap-duration>
```

For a track with only a declared synthetic pregap, the BIN begins at
`INDEX 01`, the CUE emits `PREGAP`, and `INDEX 01` is `00:00:00`.

When both forms exist, both are expressed. Index times are recalculated
relative to the per-track output BIN. Minute/second/frame values use 75 frames
per second and must be checked for overflow.

The generator must reject a layout if the canonical model cannot distinguish
or reproduce its timing. It must not invent zero-filled sectors to conceal
missing file-backed content.

### Canonical model refinement

Before implementing the encoder, evolve `CdTrack` so it explicitly retains:

* the encoded source extent beginning at the earliest preserved index;
* the file-backed pregap sector count;
* the declared synthetic pregap sector count;
* index positions in one documented coordinate system;
* the playable `INDEX 01` position within the preserved extent.

The model must not store CUE output filenames or DuckStation-specific fields.
These refinements describe CD semantics and are reusable by later Sega CD,
PC Engine CD, and other track-aware presentations.

Existing OPL logical projection continues to start at the track's playable
data and excludes pregap content.

### Serialized presentation change

The presentation schema needs a representation for one rule producing a
related file set. The intended shape is:

```yaml
version: 1
name: duckstation

layout:
  type: literal_root
  path: PS1

files:
  - select:
      type: single_disc_games_by_platform
      platform: ps1
    naming:
      type: part_name
    artifact:
      content_type: disc
      format: cue_bin
      required_features:
        - lossless
        - random_access
        - multi_file
```

The exact field spelling may be adjusted during implementation, but these
semantics are fixed:

* platform-specific selection does not require a CD/DVD guess;
* `cue_bin` means one coherent multi-file disc representation;
* track expansion belongs to encoding/materialization;
* the presentation remains serializable data.

`multi_file` describes one artifact producing related output files. It is
distinct from the existing `multi_source` capability, which describes an
encoder consuming more than one input source.

### Tracked follow-up capabilities

The capabilities deferred from the first slice are required follow-up
milestones, not an open-ended backlog:

| ID | Capability | Required outcome |
| --- | --- | --- |
| PS1-2 | CHD input | Decode complete CHD track semantics into `CdDisc` and present them through DuckStation |
| PS1-3 | SBI sidecars | Preserve source-backed SBI files and couple their basename to the presented disc |
| PS1-4 | Multi-disc M3U | Present all discs and generate a relative M3U playlist with stable names |
| PS1-5 | Cooked ISO input | Accept only the explicitly safe data-only subset without implying missing track or subchannel data |
| PS1-6a | Native CHD passthrough | Preserve an existing CHD and optional SBI byte-for-byte without re-encoding |
| PS1-6b | Materialized CHD encoding | Encode other inputs only after bounded materialization, caching, and random-access behavior are specified and measured |
| PS1-7 | Extend OPL with POPS | Add PS1 VCD content below `POPS/` to the existing OPL library presentation |

CHD input must not be represented only as a 2048-byte `LogicalDisc`; doing so
would discard the track, audio, pregap, and subchannel semantics needed by the
PS1 consumer.

Each item has acceptance criteria in the optical-media capability roadmap.
Closing the first DuckStation slice does not close the PS1 milestone group.

### OPL POPS follow-up contract

POPS support will extend the existing `opl` presentation. A user mounting an
OPL library for a PS2 should receive one coherent consumer view:

```text
DVD/   # PlayStation 2 DVD games
CD/    # PlayStation 2 CD games
POPS/  # PlayStation 1 games for the selected POPS integration
```

POPS is not a standalone presentation and is not an alias for the DuckStation
presentation. It is a new artifact rule within `opl`: PS1 discs require a
converted VCD representation and consumer-specific `POPS/` layout, while PS2
discs retain the existing ISO rules.

The VCD encoder remains an independent representation capability. The
presentation requests that capability for matching PS1 content; neither the CD
decoder nor the encoder owns the complete OPL directory layout.

The implementation must begin with a compatibility spike that selects and
records the exact supported combination of OPL, POPStarter, or POPSLoader and
storage backend. The spike must confirm:

* the VCD container header, sector, audio, and multi-track requirements;
* whether conversion can be live and random-access or requires bounded
  materialization;
* required and optional files below `POPS/`;
* game, VCD, launcher, configuration, and artwork naming coupling;
* single-disc and multi-disc behavior;
* differences between USB, MX4SIO, MMCE, HDD, and SMB where relevant;
* which launcher or Sony runtime assets must be supplied by the user and must
  never be distributed or synthesized by Retromount.

Until that spike is accepted, Retromount must not label a generic VCD or
`POPS/` layout as OPL-compatible or add an unverified POPS rule to the built-in
`opl` document.

### ZIP behavior

Filesystem and stored-ZIP CUE/BIN inputs follow the same presentation path.
Compressed ZIP track members remain rejected until the bounded random-access
cache from [ADR-011](adr-011-compressed-container-random-access.md) is
implemented.

The output remains live and does not create an extracted or converted disc.

## Non-goals

The first implementation does not add:

* CHD, ISO, ECM, MDS/MDF, CCD, PBP, or physical-disc input;
* CHD encoding;
* M3U generation or multi-disc presentation;
* SBI parsing, synthesis, or passthrough;
* subchannel synthesis or repair;
* audio decoding, resampling, or WAV output;
* merging tracks into one BIN;
* restoration of source CUE comments or cosmetic formatting;
* automatic PS1 identification from extension or disc filesystem;
* DuckStation configuration or game-list mutation;
* compressed ZIP random-access caching.

These are deferred capabilities, not reasons to weaken the canonical CD model
or the first CUE/BIN contract.

## PS1-2 implementation note

CHD CD metadata is now decoded into the same `CdDisc` contract as CUE/BIN.
Each 2448-byte CHD CD frame is exposed as a live main-sector view plus, when
declared, a separate 96-byte subchannel view. CHD-only four-frame track padding
is excluded, while physical and synthetic pregaps remain distinct.

The CUE/BIN output encoder deliberately rejects any track carrying subchannel
content. This keeps PS1-2 lossless at the canonical boundary without pretending
that CUE/BIN can represent LibCrypt or other subchannel information. SBI
sidecar presentation remains PS1-3.

## PS1-3 implementation note

SBI is modeled as optional disc-level auxiliary content. CUE and CHD decoders
look only for a sibling whose stem matches the disc input case-insensitively;
unrelated SBI files are not claimed, and multiple matching names are rejected
as ambiguous. The selected sidecar is validated as an `SBI\0` header followed
by complete 14-byte type-1 SubQ records, then retained through a live reader
handle and marked as consumed input.

The CUE/BIN encoder includes that reader in the same atomic artifact set as the
generated CUE and track BINs. Its output name is derived from the artifact
basename rather than copied from the source, guaranteeing the adjacency and
same-basename convention expected by DuckStation for both filesystem and
stored-ZIP sources.

## PS1-4 implementation note

Multi-disc expansion is selected declaratively by platform, but compiled as
one atomic game directory rather than unrelated per-disc rules. Disc numbers
parsed by the dedicated CUE and CHD decoders feed normalization and stable
ordering. Each part then reuses the existing CUE/BIN artifact-set encoder in a
nested directory.

The generated M3U is a sibling of those disc directories and references the
planned CUE member of each set with a forward-slash relative path. The outer
game name is conflict-resolved once before any children are planned, keeping
the complete set coherent if another game requests the same presentation name.

## PS1-5 implementation note

Cooked ISO input is accepted only in a composition that supplies explicit CD
media and PS1 platform context. The ISO decoder represents its complete
2048-byte-sector extent as one `MODE1/2048` data track with `INDEX 01` at zero.
The original random-access reader is retained for both encoded and logical
content, allowing the existing DuckStation CUE/BIN encoder to expose the bytes
without manufacturing raw-sector headers or claiming absent audio, pregap, or
subchannel information.

The decoder rejects empty and partial-sector images. Filesystem and stored-ZIP
sources use the same live reader contract. OPL continues to compose the ISO
decoder with its own explicit or default CD/DVD media selection, so adding the
PS1 path does not change PS2 presentation semantics.

## PS1-6a implementation note

Native CHD presentation is source passthrough, not CHD encoding. Serialized
presentation rules can include or exclude source formats, allowing DuckStation
to request a CHD artifact set only when every selected disc already originates
from CHD. Other inputs continue to request CUE/BIN.

The passthrough encoder validates that every canonical track points to one CHD
source, exposes that source byte-for-byte, and retains an optional SBI under the
same generated basename. Multi-disc CHD games use one atomic game directory,
per-disc CHD artifact sets, and a relative M3U referencing the native CHDs.
Embedded subchannel data remains intact because the CHD is never projected
through the more limited CUE/BIN representation.

Encoding CUE/BIN or ISO input to CHD is PS1-6b. It is not a live transform: it
requires a completed materialized file with known length and usable random
access. Its temporary-storage, cancellation, concurrency, invalidation, and
reuse contract belongs to the performance and caching milestone.

## Consequences

### Positive

* The first PS1 presentation has a testable consumer contract.
* Mixed-mode and audio discs exercise the track-aware architecture directly.
* Output is lossless and reader-backed without a converted whole-disc image.
* Generated CUE references cannot leak source or container paths.
* Artifact-set planning is reusable for sidecars and other multi-file formats.
* The CD model gains the timing precision required by future platforms.

### Negative

* Presentation planning and materialization must support one-to-many output.
* The current combined pregap model requires refinement.
* Separate per-track BIN files differ from some source CUE file grouping.
* The first slice does not immediately make existing CHD or ISO collections
  available to the PS1 presentation.
* Extending OPL with POPS needs its own researched conversion and filesystem
  contract.

## Implementation sequence

1. Refine `CdTrack` pregap, index, and encoded-extent semantics, preserving OPL
   behavior and adding mixed-mode regression coverage.
2. Add serialized platform-only single-disc selection.
3. Add the `cue_bin` format and multi-file artifact-set planning contract.
4. Implement bounded per-track encoded readers and deterministic set naming.
5. Generate CUE documents from canonical track timing.
6. Add `presentations/duckstation.yaml` to the built-in catalog.
7. Add integration fixtures for:
   * single-track raw data;
   * mixed data and audio tracks;
   * file-backed `INDEX 00`;
   * declared `PREGAP`;
   * single-file and multi-file source CUE layouts;
   * filesystem and stored-ZIP parity;
   * PS2 and unknown-platform exclusion;
   * atomic naming-conflict handling;
   * arbitrary and unaligned BIN reads.
8. Validate the mounted result with DuckStation on a representative,
   user-owned PS1 image and document the result.

## Implementation status

The software implementation is complete. It includes:

* canonical file-backed and declared pregap separation;
* track-relative indexes and complete encoded source extents;
* platform-specific single-disc selection;
* atomic multi-file artifact-set planning and materialization;
* a lossless, random-access CUE/BIN encoder;
* generated CUE timing and sibling per-track BIN names;
* an embedded `duckstation` presentation rooted at `PS1/`;
* mixed-mode filesystem and stored-ZIP integration coverage.

Representative real-DuckStation validation remains to be documented before
the milestone is considered operationally verified.

[duckstation-readme]: https://github.com/stenzek/duckstation/blob/master/README.md
[duckstation-sbi]: https://github.com/stenzek/duckstation/blob/master/README.md#libcrypt-protection-and-sbi-files
