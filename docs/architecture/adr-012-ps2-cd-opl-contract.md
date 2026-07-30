# ADR-012: Present Cooked PlayStation 2 CDs to OPL

## Status

Accepted

## Context

Retromount can currently present PlayStation 2 DVD content to Open PS2 Loader
(OPL) as a live `DVD/<title>.iso` file. The next optical-media slice must add
PlayStation 2 CD support without treating every CD as a contiguous DVD-like
image or confusing PlayStation and PlayStation 2 media.

The current OPL implementation establishes two relevant parts of its consumer
contract:

* OPL scans `CD/` and `DVD/` separately and assigns the `SCECdPS2CD` or
  `SCECdPS2DVD` media value according to the directory being scanned.
* A plain PlayStation 2 CD image must be an ISO9660 image with 2048-byte
  sectors. Raw BIN/CUE images with a different sector size must be converted
  before OPL can consume them as plain ISO files.

OPL accepts modern `<name>.iso` filenames and reads `SYSTEM.CNF` to obtain the
startup executable. It also accepts the older
`<game-id>.<name>.iso` convention. Current OPL source permits a modern name of
up to 160 characters before the extension, although Retromount's existing
naming and conflict policies remain responsible for producing safe names.

Primary references:

* [OPL README directory and format contract][opl-readme]
* [OPL USB-mode CD image guidance][opl-usb]
* [OPL ISO scanning and media assignment][opl-supportbase]
* [OPL game-name limits][opl-supportbase-header]

An ISO image does not reliably encode whether its original physical carrier
was a CD or DVD. Its extension, ISO9660 filesystem, and byte length are
insufficient to classify the medium without heuristics. Retromount already
requires platform context to remain distinct from input format; media context
needs the same treatment for an ISO input.

The current serialized OPL presentation has a single literal `DVD` root. That
shape cannot express sibling `CD/` and `DVD/` directories. Implementing the CD
layout as Rust-only presentation logic would undermine the decision to make
consumer presentations declarative.

## Decision

The first PlayStation 2 CD vertical slice will support a cooked, single-track
2048-byte ISO as a live logical CD and present it to OPL without changing its
bytes.

### Input contract

The initial supported input must:

* be identified as an ISO input;
* provide efficient random access through `InputContent`;
* have a non-zero length that is an exact multiple of 2048 bytes;
* receive explicit `Platform::Ps2` and `DiscMedia::Cd` context from application
  composition.

The ISO decoder must not classify CD versus DVD from extension or size.

The platform and media hints are trusted composition inputs in this slice.
Content probing may later validate or derive those values, but must not silently
override explicit conflicting context.

Stored ZIP entries may use the same decoder because they provide efficient
random access. Compressed ZIP ISO entries retain the early rejection defined by
[ADR-011](adr-011-compressed-container-random-access.md).

### Canonical media contract

The existing `LogicalDisc` is sufficient for this initial CD subset:

```text
media:       CD
sector_size: 2048
sector_count: input length / 2048
content:     live random-access input handle
```

This does not establish `LogicalDisc` as the representation for all CDs. It is
valid only because this slice accepts a single cooked data track whose complete
consumer-visible representation is the contiguous 2048-byte logical sector
stream.

### OPL output contract

For a normalized single-disc PlayStation 2 game whose logical media is CD, the
OPL presentation will request:

```text
CD/<title>.iso
```

The ISO artifact must:

* have exactly the same logical length as the input;
* map every output range to the same input bytes;
* remain reader-backed and random-access;
* use no whole-disc conversion or intermediate.

The existing PlayStation 2 DVD rule remains:

```text
DVD/<title>.iso
```

The selectors must fail closed:

* PlayStation 2 DVDs appear only below `DVD/`;
* PlayStation 2 CDs appear only below `CD/`;
* PlayStation 1 and unknown-platform CDs match neither OPL rule.

### Declarative presentation change

The presentation schema will gain an optional per-file-rule destination
directory. The OPL document can then use a root-level layout with two rules:

```yaml
files:
  - directory: DVD
    select:
      type: single_disc_games_by_platform_and_media
      platform: ps2
      media: dvd
    # naming and artifact omitted

  - directory: CD
    select:
      type: single_disc_games_by_platform_and_media
      platform: ps2
      media: cd
    # naming and artifact omitted
```

The exact root-layout spelling may be chosen during implementation, but the
directory must remain serialized presentation data rather than OPL-specific
Rust branching.

Destination paths must be validated as relative virtual paths. Absolute paths,
parent traversal, empty segments, and platform-dependent separators must be
rejected or normalized according to one documented virtual-path rule.

### Composition change

Decoder registration must no longer hard-code every ISO as DVD merely because
the selected presentation is OPL.

The initial application-facing route will allow a view or equivalent
composition boundary to provide an explicit CD/DVD media hint. A single-source
PS2 CD view can therefore select the CD ISO decoder without guessing.

Supporting a mixed input directory containing ambiguous CD and DVD ISO files
requires per-source metadata or reliable content identification. It is not
part of this initial slice.

## Non-goals

This decision does not add:

* raw 2352-byte MODE1 or MODE2 BIN conversion;
* CUE/BIN input for the OPL CD path;
* mixed-mode or audio-track support;
* PlayStation 1 input or presentation support;
* automatic CD/DVD classification by image size;
* automatic platform classification solely from `SYSTEM.CNF`;
* OPL ZSO output;
* compressed ZIP random-access caching;
* SMB or physical-console OPL integration testing.

Raw and multi-track CD inputs must wait for a track-aware canonical model. They
must not be forced through `LogicalDisc` by discarding headers, subchannel
information, audio, pregaps, indexes, or track boundaries.

## Consequences

### Positive

* The first PS2 CD implementation remains a small extension of the proven live
  ISO path.
* OPL receives the exact directory and sector representation it expects.
* CD/DVD classification is explicit rather than based on fragile size
  heuristics.
* PS1 and unknown CDs do not leak into the OPL view.
* The presentation-schema change is consumer-neutral and supports future
  multi-directory presentations.
* The richer PS1 slice remains free to introduce track-aware media semantics.

### Negative

* Users must provide CD media context for ambiguous ISO input.
* A mixed directory of unlabelled PS2 CD and DVD ISOs cannot be classified
  automatically in this slice.
* Common raw BIN/CUE PS2 CD dumps remain unsupported initially.
* The serialized presentation schema and compiler need a small extension before
  the OPL YAML can contain both rules.

## Implementation sequence

1. Add and validate per-rule destination directories in serialized
   presentations.
2. Rewrite the OPL YAML to declare sibling `DVD/` and `CD/` rules while
   preserving existing DVD behavior.
3. Move ISO media selection from presentation-name branching to explicit
   composition context.
4. Decode a 2048-byte ISO as `LogicalDisc { media: Cd, ... }`.
5. Allow the logical-disc ISO encoder to pass through the supported CD subset.
6. Add unit and integration coverage for CD/DVD separation, PS1/unknown
   exclusion, arbitrary unaligned reads, exact length, and stored ZIP parity.

[opl-readme]: https://github.com/ps2homebrew/Open-PS2-Loader#how-to-use
[opl-usb]: https://github.com/ps2homebrew/Open-PS2-Loader/wiki/usb-mode#installing-ps2-games
[opl-supportbase]: https://github.com/ps2homebrew/Open-PS2-Loader/blob/master/src/supportbase.c
[opl-supportbase-header]: https://github.com/ps2homebrew/Open-PS2-Loader/blob/master/include/supportbase.h
