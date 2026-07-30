# ADR-012: Decode PlayStation 2 CDs and Present the OPL-Compatible Subset

## Status

Accepted

## Context

Retromount can present PlayStation 2 DVD content to Open PS2 Loader (OPL) as a
live `DVD/<title>.iso` file. The next optical-media slice must add PlayStation
2 CD support without treating every CD as a contiguous DVD-like image or
confusing PlayStation and PlayStation 2 media.

The current OPL implementation establishes two relevant consumer requirements:

* OPL scans `CD/` and `DVD/` separately and assigns the `SCECdPS2CD` or
  `SCECdPS2DVD` media value according to the scanned directory.
* A plain PlayStation 2 CD image must be an ISO9660 image with 2048-byte
  sectors. Raw BIN/CUE images with a different sector size must be converted
  before OPL consumes them as plain ISO files.

OPL accepts modern `<name>.iso` filenames and reads `SYSTEM.CNF` to obtain the
startup executable. It also accepts the older
`<game-id>.<name>.iso` convention. Current OPL source permits a modern name of
up to 160 characters before the extension, although Retromount's naming and
conflict policies remain responsible for safe names.

Primary references:

* [OPL README directory and format contract][opl-readme]
* [OPL USB-mode CD image guidance][opl-usb]
* [OPL ISO scanning and media assignment][opl-supportbase]
* [OPL game-name limits][opl-supportbase-header]

An ISO image does not reliably encode whether its original physical carrier
was a CD or DVD. Its extension, ISO9660 filesystem, and byte length are
insufficient to classify the medium without heuristics. Retromount already
keeps platform context distinct from input format; media context needs the same
treatment for ISO input.

Common CD dumps are CUE/BIN rather than cooked ISO. They may contain raw
2352-byte data sectors, multiple files, multiple data or audio tracks, pregaps,
and index points. Retromount needs to decode and preserve those layouts for
future PS1 and other CD-based presentations even though OPL cannot represent
all of them.

The current serialized OPL presentation has one literal `DVD` root. That shape
cannot express sibling `CD/` and `DVD/` directories. Implementing the CD layout
as Rust-only presentation logic would undermine declarative presentations.

## Decision

The PlayStation 2 CD slice will introduce a live, track-aware CD model and
support both cooked ISO and CUE/BIN input. It will present only the subset that
can be represented correctly by OPL's contiguous 2048-byte ISO contract.

Supporting an input does not imply that every consumer can represent it.
Retromount must preserve complete decoded CD semantics even when the OPL
presentation rejects the disc.

### ISO input contract

An ISO input must:

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

### CUE/BIN input contract

CUE/BIN decoding must use structured source resolution and live
`InputContent` handles. The CUE decoder owns layout interpretation; it must not
assume that referenced BIN files are filesystem paths or parse ZIP addressing.

The supported CUE subset will preserve:

* ordered track numbers and types;
* each track's source and byte range;
* sector encoding and encoded sector size;
* `INDEX 00` and `INDEX 01` positions;
* declared and file-backed pregaps;
* audio tracks without decoding or resampling their PCM payload;
* the distinction between single-file and multi-file layouts.

Unsupported or contradictory CUE directives must produce an actionable error.
They must not be ignored when doing so would alter track boundaries or timing.

### Raw sector mapping

Raw data sectors may provide a live logical 2048-byte projection when their
encoding makes that projection unambiguous:

| CUE track mode | Encoded bytes | Logical user bytes | Initial action |
| --- | ---: | ---: | --- |
| `MODE1/2048` | 2048 | 2048 | identity |
| `MODE1/2352` | 2352 | 2048 | map validated user-data range |
| `MODE2/2352` Form 1 | 2352 | 2048 | map validated user-data range |
| `MODE2/2352` Form 2 | 2352 | 2324 | preserve; not ISO-compatible |
| `AUDIO` | 2352 | n/a | preserve; not ISO-compatible |

The mapper must validate sector sync, mode, and form information instead of
blindly stripping a fixed prefix. It must support arbitrary unaligned reads
across logical sector boundaries without materializing the whole track.

Raw sector projection is a consumer view over preserved encoded sectors. It
does not replace the canonical track or discard headers, error-detection data,
error-correction data, subheaders, audio, or layout metadata.

### Canonical media contract

The canonical CD model will describe the disc and its ordered tracks. At
minimum, each track needs:

* track number and data/audio kind;
* encoded sector format and sector size;
* sector count and source byte range;
* index and pregap metadata;
* a live encoded-sector reader;
* an optional live logical user-data reader when a safe projection exists.

The existing path-based `Disc`, `Track`, and `TrackSource` types predate live
input content and are not sufficient as the canonical contract. They will be
replaced or evolved so ZIP-backed and filesystem-backed tracks use the same
reader capabilities.

`LogicalDisc` remains the contiguous consumer-ready view for a disc with one
safe logical data stream. For a cooked ISO or eligible single data track:

```text
media:       CD
sector_size: 2048
sector_count: projected logical length / 2048
content:     live random-access logical reader
```

Track-aware content remains available alongside that projection. A
`LogicalDisc` must not be created when doing so would imply that audio,
additional data tracks, non-2048-byte user data, or required layout semantics
can be discarded.

### OPL output contract

For a normalized single-disc PlayStation 2 game whose CD has an OPL-compatible
logical projection, the OPL presentation will request:

```text
CD/<title>.iso
```

The ISO artifact must:

* have exactly the projected logical length;
* map every output range to the corresponding 2048-byte user data;
* remain reader-backed and random-access;
* use no whole-disc conversion or intermediate.

Cooked 2048-byte ISO input is byte-identical. Raw 2352-byte input is a live
sector projection, so it is not byte-identical to the encoded source, but it is
an exact mapping of its validated logical user data.

The existing PlayStation 2 DVD rule remains:

```text
DVD/<title>.iso
```

The selectors and encoder must fail closed:

* PlayStation 2 DVDs appear only below `DVD/`;
* OPL-compatible PlayStation 2 CDs appear only below `CD/`;
* PlayStation 1 and unknown-platform CDs match neither OPL rule;
* mixed-mode, audio, Form 2, and otherwise non-projectable CDs fail explicitly
  rather than producing a partial ISO.

### Declarative presentation change

The presentation schema will gain an optional per-file-rule destination
directory. The OPL document can then declare:

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
directory remains serialized presentation data rather than OPL-specific Rust
branching.

Destination paths must be validated as relative virtual paths. Absolute paths,
parent traversal, empty segments, and platform-dependent separators must be
rejected or normalized by one documented virtual-path rule.

### Composition change

Decoder registration must no longer hard-code every ISO as DVD merely because
the selected presentation is OPL.

The application-facing route will allow a view or equivalent composition
boundary to provide an explicit CD/DVD media hint. A single-source PS2 CD view
can therefore select the CD ISO decoder without guessing.

A mixed input directory containing ambiguous CD and DVD ISO files still
requires per-source metadata or reliable content identification. CUE metadata
establishes track layout but does not establish PS1 versus PS2 identity.

## Non-goals

This decision does not add:

* lossy flattening of mixed-mode or audio CDs for OPL;
* audio decoding, resampling, or generated audio files;
* synthesis or repair of missing raw-sector EDC/ECC data;
* subchannel data absent from the selected input;
* multi-session support unless required by an accepted fixture;
* a PlayStation 1 consumer presentation;
* automatic CD/DVD classification by image size;
* automatic platform classification solely from `SYSTEM.CNF`;
* OPL ZSO output;
* compressed ZIP random-access caching;
* SMB or physical-console OPL integration testing.

This slice establishes the track-aware model needed by later PS1 and other
CD-based presentations. It does not assume those consumers want OPL's
2048-byte projection.

## Consequences

### Positive

* Cooked and common raw PS2 CD inputs share one preserved media model.
* OPL receives the directory and sector representation it expects.
* CD/DVD classification is explicit rather than based on size heuristics.
* PS1 and unknown CDs do not leak into the OPL view.
* Mixed-mode and audio information survives for later presentations.
* Raw-sector conversion remains live and bounded.
* The presentation change supports future multi-directory consumers.

### Negative

* Users must provide CD media context for ambiguous ISO input.
* A mixed directory of unlabelled CD and DVD ISOs cannot be classified
  automatically in this slice.
* CUE parsing, track validation, and raw-sector mapping materially increase the
  milestone's scope.
* OPL cannot consume every CD that Retromount can decode.
* The serialized presentation schema and compiler need an extension before the
  OPL YAML can contain both rules.

## Implementation sequence

1. Add and validate per-rule destination directories in serialized
   presentations.
2. Rewrite the OPL YAML to declare sibling `DVD/` and `CD/` rules while
   preserving existing DVD behavior.
3. Define the live track-aware CD model and migrate existing CUE structures
   away from path-only track sources.
4. Decode CUE/BIN while preserving tracks, indexes, pregaps, source ranges, and
   audio metadata.
5. Add bounded live mappers for MODE1/2352 and MODE2/2352 Form 1 user data.
6. Move ISO media selection from presentation-name branching to explicit
   composition context.
7. Decode a 2048-byte ISO as `LogicalDisc { media: Cd, ... }`.
8. Expose `LogicalDisc` only for the OPL-compatible CUE/BIN subset.
9. Allow the logical-disc ISO encoder to emit supported CD projections.
10. Add unit and integration coverage for:
    * CD/DVD separation and PS1/unknown exclusion;
    * cooked ISO identity reads;
    * unaligned reads across raw-sector boundaries;
    * MODE1 and MODE2 Form 1 mapping;
    * Form 2 rejection;
    * multi-file and single-file CUE layouts;
    * index, pregap, mixed-mode, and audio preservation;
    * explicit OPL rejection of non-projectable layouts;
    * stored ZIP parity and compressed ZIP rejection.

[opl-readme]: https://github.com/ps2homebrew/Open-PS2-Loader#how-to-use
[opl-usb]: https://github.com/ps2homebrew/Open-PS2-Loader/wiki/usb-mode#installing-ps2-games
[opl-supportbase]: https://github.com/ps2homebrew/Open-PS2-Loader/blob/master/src/supportbase.c
[opl-supportbase-header]: https://github.com/ps2homebrew/Open-PS2-Loader/blob/master/include/supportbase.h
