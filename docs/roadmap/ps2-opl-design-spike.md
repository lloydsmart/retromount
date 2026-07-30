# PS2/OPL Design Spike

## Status

Revised proposed contract and capability assessment.

This spike follows Phase 6 and defines the smallest useful PS2/Open PS2 Loader
(OPL) target. It is a design input for implementation, not a claim that the
target currently works end to end.

## Decision summary

The first target is:

* one CHD containing a single PS2 DVD image;
* one OPL view with one output at `DVD/<game title>.iso`;
* live, random-access reads of the logical disc data stored in the CHD;
* bounded in-memory caching of individual decompressed hunks only;
* preservation of the generic input → decode → normalize → encode → output
  pipeline.

The first target explicitly excludes CD games, multi-disc games, parent/delta
CHDs, USBExtreme splitting, ZSO output, game artwork and configuration, and
automatic extraction of a PS2 game ID.

The PS2/OPL target is a presentation specification, not a conversion workflow.
CHD is one possible input container, and ISO is one possible output
representation. They are selected and implemented independently.

Whole-image extraction is prohibited for this target. The implementation must
not invoke an extraction command, create a temporary ISO, create a persistent
ISO cache, or require an ISO-sized intermediate artifact. Its behavior should
match the live, random-access model exemplified by `chd2iso-fuse`.

## Architectural invariant

The implementation must preserve this composition:

```text
Input source
    ↓
Identify input container
    ↓
Decode media and geometry to canonical logical disc + random-access content
    ↓
Normalize game/platform/disc metadata
    ↓
Compile OPL presentation requirements
    ↓
Resolve an ISO output encoder
    ↓
Materialize a reader-backed VFS file
    ↓
OPL reads arbitrary ISO ranges
```

No component should implement the whole path from CHD input to OPL layout:

* the CHD decoder must not know about OPL paths or names;
* the OPL presentation must not know about CHD;
* the ISO encoder must not require CHD as its input;
* the VFS must not know which input container supplies the bytes.

This allows the same decoded CHD disc to be used by other presentations and
encoders, and allows the OPL presentation to accept other decoders that produce
the same canonical logical-disc contract.

## OPL output contract

### Filesystem shape

The root presented to OPL is an OPL share root:

```text
/
└── DVD/
    └── Shadow of the Colossus.iso
```

For the first target:

* `DVD` is a literal, case-sensitive directory at the view root;
* exactly one regular file is emitted below it;
* the file extension is exactly `.iso`;
* no playlist is emitted;
* the ISO is read-only;
* the reported size is the exact logical ISO byte length;
* reads at arbitrary offsets return the corresponding logical ISO bytes;
* no complete ISO or ISO-sized intermediate is written before or during the
  mount.

The presentation contract is consumer-facing. It does not name CHD, a CHD
library, or a specific encoder implementation.

### Selection

The presentation selects only a normalized game that:

* is identified as PlayStation 2;
* contains exactly one disc part;
* represents DVD media;
* has a canonical logical data view that can satisfy an ISO artifact request.

Selection must fail closed. Content with unknown platform or unknown media type
must not silently appear in the OPL `DVD/` directory.

### Naming

The first target uses:

```text
<sanitized game title>.iso
```

The output-format extension replaces any source or policy-derived extension.

The first slice does not require OPL's optional game-ID-prefixed form. A later
profile may produce:

```text
<game ID>.<sanitized game title>.iso
```

when reliable game-ID extraction and an explicit length policy exist.

Name allocation must be deterministic and must reject or resolve collisions
before materialization starts.

### Artifact request

The compiled `PresentationPlan` should contain the equivalent of:

```text
path: DVD/Shadow of the Colossus.iso
kind: content-backed
input: canonical logical disc handle
requirements:
  content_type: disc
  output_format: iso
  media: dvd
  required_features:
    - random_access
    - lossless
```

The request describes the desired output. It contains no CHD-specific
requirement.

### Encoder capability

The ISO encoder should advertise a representation capability such as:

```text
capability_id: disc.logical-to-iso.dvd
content_type: disc
output_formats:
  - iso
accepted_inputs:
  representations:
    - logical_dvd_data
  media:
    - dvd
features:
  - random_access
  - lossless
```

The accepted input is a canonical representation, not a storage format. The
same encoder should work with logical DVD data produced from CHD, ISO, or any
future supported container.

For a DVD whose canonical logical view is already a contiguous stream of
2048-byte sectors, ISO materialization can be a lightweight reader adapter
rather than a byte conversion.

## Stage contracts

### Identify

Identification determines that a source is a CHD container. It may also extract
cheap header information, but it does not choose an output format or consumer.

### Decode

The CHD decoder:

* validates the CHD header and metadata;
* determines the media type and logical geometry;
* exposes the disc's canonical logical data;
* provides random-access reads by decompressing only the hunks needed for a
  requested range;
* may use a bounded in-memory cache of decompressed hunks;
* never extracts or writes a complete ISO;
* does not choose an output name or directory.

The decoder output needs a content handle that can open or retain a
random-access reader. A bare `SourceRef` is insufficient because it describes
where encoded input lives, not how to read decoded logical content.

Conceptually:

```text
DecodedDisc {
  media: DVD
  logical_sector_size: 2048
  logical_sector_count: N
  content: ContentHandle
}

ContentHandle.open() -> Reader
Reader.len() -> N * 2048
Reader.read_at(offset, buffer) -> logical disc bytes
```

### Normalize

Normalization adds stable semantic information:

* title;
* platform;
* disc number;
* media type;
* canonical disc representation and geometry;
* the opaque content handle.

It must not discard the decoded content handle and regress to the original CHD
path. It also must not contain OPL layout or ISO naming decisions.

### Present

The OPL specification:

* selects a single PS2 DVD game;
* places it below literal directory `DVD`;
* derives the consumer-facing name;
* requests `Disc + ISO + DVD + RandomAccess + Lossless`.

It does not inspect `.chd` extensions and does not select a named encoder.

### Encode

The ISO encoder consumes the normalized canonical logical-disc representation.
It:

* verifies DVD media and ISO-compatible logical sector geometry;
* calculates the exact output length;
* returns a reader-backed view over the canonical disc content;
* maps requested byte ranges to the underlying logical-disc reader;
* remains independent of the original input container.

For the first DVD target this can be an identity mapping over logical sector
bytes. The encoder boundary is still valuable: later disc representations may
need framing removal, track selection, padding, or another output-specific
mapping without changing the presentation or decoder.

### Output/VFS

The VFS must support a materialized artifact backed by a live reader, in
addition to inline and path-backed files.

Conceptually:

```text
MaterializedArtifact::ReaderBacked {
  handle: ReaderHandle,
  size: u64,
}
```

The handle should be opaque and resolved through a mount-session resource
registry or equivalent lifetime owner. It should not serialize a Rust trait
object into the plugin protocol.

The mount session must:

* keep the decoded content and reader factory alive;
* open or reuse readers safely;
* forward FUSE offset reads without whole-file materialization;
* support concurrent or repeated reads;
* release resources when the session ends.

## Random-access read behavior

A read of output range `[offset, offset + length)` should:

1. validate and clamp the request against the logical ISO length;
2. identify the logical sectors and CHD hunks covering that range;
3. fetch and decompress only those hunks not already in the bounded cache;
4. copy only the requested bytes into the output buffer;
5. return without creating an extracted ISO file.

The optional hunk cache is an implementation detail of the CHD reader, not an
output artifact:

* it should be bounded;
* it may retain decompressed hunks in memory;
* it must not change the observable ISO bytes;
* it must not be required for correctness;
* it can be shared where safe across readers for the same decoded content.

It must never contain or grow into a complete ISO representation.

## Existing capability assessment

### What already fits

The current architecture has several correct foundations:

* `Reader` already defines `read_at(offset, buffer)` and `len()`;
* readers are explicitly read-only;
* `ReaderFactory` and `ReaderRegistry` already establish reader construction as
  a distinct concern;
* `PresentationSpec` can request `ContentType::Disc` and `Format::Iso`;
* `PresentationPlan` separates artifact intent from materialization;
* capability resolution selects encoders independently of presentations;
* FUSE already forwards offset reads through the reader abstraction.

These are the pieces to extend. They should not be replaced by an extraction
workflow.

### Blocking gaps

| Area | Current behavior | Required change |
| --- | --- | --- |
| CHD input | No CHD reader or decoder exists. | Add a random-access CHD reader that exposes logical disc bytes and media metadata. |
| Input identification | Only `.cue` is identified as a disc image; `.chd` becomes generic bytes. | Identify CHD as a disc container and route it to the CHD decoder. |
| Decoded content | `DecodedDiscContent` retains source metadata but no live decoded-content handle or geometry. | Carry a reusable canonical logical-disc handle, media kind, sector size, and sector count. |
| Normalized content | `DiscPart` retains only source, disc number, and consumed sources. | Preserve the canonical representation and content handle through normalization. |
| Platform model | The active normalized `Platform` enum has no PS2 variant. | Add PS2 and derive it from an explicit hint or reliable identification. |
| Presentation placement | Layouts are only flat or grouped by platform/game. | Express literal `DVD/` placement declaratively. |
| Presentation selection | Rules cannot filter by platform, media, or representation. | Add declarative filters needed by the OPL rule. |
| Naming | `PartName` produces `.cue`, and the compiler preserves existing extensions. | Make output-format extension replacement explicit. |
| Capability model | Capabilities match output content type and format, but not canonical input representation or media. | Match encoders against normalized representation and media, not source container extension. |
| Materialized output | Artifacts can only be inline or source-path-backed. | Add a reader-backed artifact and mount-session lifetime owner. |
| VFS model | `VfsFile` derives serializable/value equality over path or bytes. | Separate inspectable file metadata from non-serializable runtime backing handles. |
| Plugin protocol | The subprocess model is one request/one response and can return only bytes or a path. | Do not force live readers through protocol v1; design a persistent/random-read plugin contract later if external reader plugins are required. |
| Validation | No synthetic PS2 DVD CHD fixture or live-read test exists. | Verify geometry, exact length, unaligned reads, hunk-boundary reads, repeated reads, and final OPL path. |

## Plugin boundary conclusion

Runtime encoder plugins remain the output extensibility mechanism, but protocol
v1 cannot transport a live random-access reader. This spike must not work around
that limitation by materializing a path-backed ISO. Doing so would optimize for
the existing protocol at the expense of the architecture.

For the first target:

* CHD decoding should use the input/reader side of the architecture;
* the ISO encoder should operate on the canonical logical-disc handle;
* reader-backed materialization should be an in-process host capability;
* OPL remains a declarative presentation.

If external CHD decoders or live output encoders are required later, they need a
persistent protocol with open/read/close semantics or another shared
random-access backing mechanism. That is a separate input/runtime-plugin design
problem and should not be hidden inside a PS2-specific encoder.

## Recommended implementation order

1. Define the canonical logical-disc representation and opaque content-handle
   lifetime.
2. Add reader-backed materialized artifacts and mount-session resource
   ownership, proven first with a simple test reader.
3. Add PS2, DVD media, geometry, and canonical representation metadata to
   decode and normalization.
4. Implement a random-access CHD reader with bounded decompressed-hunk caching.
5. Add the generic logical-DVD-to-ISO encoder adapter.
6. Add the declarative OPL `DVD/` presentation and correct extension handling.
7. Prove one synthetic DVD CHD end to end through inspect, mount preparation,
   unaligned reads, hunk-boundary reads, and reads near end-of-file.
8. Validate the mounted share with a current OPL build over SMB.

## Acceptance criteria for the first target

The spike is implemented when:

* a PS2 DVD CHD is decoded into a canonical logical-disc handle;
* no complete ISO or ISO-sized intermediate is created;
* the OPL presentation compiles exactly one
  `Disc + ISO + DVD + RandomAccess + Lossless` request at
  `DVD/<title>.iso`;
* resolution selects an encoder based on canonical representation and media,
  independently of CHD;
* the materialized VFS file reports the exact non-zero logical ISO size;
* arbitrary, unaligned, repeated, and out-of-order reads return correct bytes;
* reads spanning CHD hunk boundaries return correct bytes;
* memory use is bounded independently of total ISO size;
* invalid CHD, unsupported media, decompression failure, and out-of-range reads
  produce actionable behavior;
* an OPL SMB client lists and launches the test image;
* no fixed CHD→OPL pipeline, extraction command, or whole-image cache is
  introduced.

## Deferred questions

These should not expand the first target:

* CD track selection and conversion to a 2048-byte-sector ISO view;
* DVD9 and layer-break validation;
* parent/delta CHDs;
* CHDs inside ZIP or other virtual sources;
* external/persistent reader plugin transport;
* cross-session decompressed-hunk caching;
* game-ID extraction and optional OPL-prefixed names;
* FAT32/USBExtreme splitting;
* ZSO output;
* multi-disc presentation and OPL per-game metadata.

## External references

* [Open PS2 Loader README](https://github.com/ps2homebrew/Open-PS2-Loader)
  documents the `CD` and `DVD` directory layout and ISO support for USB and SMB.
* [OPL USB mode documentation](https://github.com/ps2homebrew/Open-PS2-Loader/wiki/usb-mode)
  documents DVD/CD placement, the optional game-ID naming form, and FAT
  filesystem size limits.
