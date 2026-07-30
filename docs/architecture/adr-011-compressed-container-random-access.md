# ADR-011: Cache Non-Seekable Container Entries on Disk

## Status

Accepted

## Context

Retromount can expose disc content through live, random-access readers without
creating a converted copy of the output image. This is required for workloads
such as CHD-to-logical-ISO presentation, where consumers issue arbitrary and
out-of-order reads.

The live input-content boundary distinguishes between inputs that provide
efficient random access and inputs that are effectively sequential:

* filesystem files provide random access;
* stored ZIP entries provide random access into the archive file;
* Deflate-compressed ZIP entries are stream-oriented.

Reading an arbitrary offset from a Deflate stream normally requires
decompressing from the beginning of that stream. Repeating this operation for
disc access patterns would produce unacceptable latency and repeated CPU work.
Keeping the complete decompressed entry in memory would violate Retromount's
bounded-memory goals.

The current implementation therefore supports compressed ZIP entries for
ordinary ROM, text, and other sequential-friendly paths, but rejects compressed
ISO and CHD entries whose decoders require efficient random access.

This creates an apparent tension with the existing rule that Retromount must
not create a whole-disc intermediate. A distinction is needed between:

* converting decoded media into a consumer output ahead of the mount; and
* making the encoded source entry seekable so its decoder can operate.

## Decision

Retromount may use a managed, disk-backed source-entry cache to make an
otherwise non-seekable container entry available to a random-access decoder.

This is a source/container concern. It is not an encoder, presentation, or
media-conversion mechanism.

### Current behavior

Until the cache is implemented:

* stored ZIP entries may satisfy random-access decoder requirements;
* compressed ZIP entries remain supported for content paths that do not
  require efficient random access;
* compressed ISO and CHD entries fail early with an actionable unsupported
  error;
* Retromount must not silently extract a compressed disc entry.

Diagnostics should explain that users can repackage the entry with ZIP's
`stored` method to obtain immediate live support.

### Initial cache strategy

The first implementation should use eager, disk-backed entry materialization:

1. resolve the requested container entry;
2. validate configured cache policy and available budget;
3. decompress the encoded entry into a temporary file;
4. verify its expected length and container checksum;
5. publish the completed cache entry atomically;
6. expose the cache file through the normal random-access input handle.

The initial implementation should prefer predictable behavior over speculative
lazy decompression.

For a Deflate-compressed ISO, the cache entry is the complete ISO source entry.
For a Deflate-compressed CHD, the cache entry is the CHD source entry, not a
decoded or converted ISO.

### Cache requirements

The cache must:

* be explicitly enabled rather than silently consuming disk;
* have a configurable total disk budget;
* reject an entry that cannot fit within the permitted budget;
* never retain the complete entry solely in memory;
* use stable invalidation inputs such as archive identity, member name,
  compressed and uncompressed sizes, CRC, and archive modification metadata;
* prevent incomplete files from being observed as valid entries;
* coordinate concurrent requests for the same entry;
* define lifecycle and eviction behavior;
* provide actionable diagnostics for materialization and verification failure;
* remain independent of the requested presentation and output encoder.

Cache entries may be reused across mount sessions when persistent caching is
enabled and validation succeeds.

### Relationship to the no-intermediate rule

Retromount continues to prohibit whole-image output conversion as a hidden
mount prerequisite. In particular, CHD-to-ISO presentation must not create or
cache a decoded ISO.

Materializing an encoded container member is permitted only because the source
container cannot provide the access semantics required by its decoder. The
cached bytes must be byte-equivalent to the original uncompressed member.

```text
Allowed:
  deflated ZIP member → cached original ISO member → ISO decoder
  deflated ZIP member → cached original CHD member → CHD decoder

Prohibited:
  CHD → cached converted ISO → OPL presentation
```

### Deferred strategies

Lazy materialization may be considered after the eager cache is implemented
and measured. It would progressively decompress into a disk-backed file and
coordinate readers waiting for later ranges.

Deflate seek indexing may also be investigated later. Such an implementation
would need restart checkpoints, dictionary state, compressed bit positions, and
bounded index storage. It is not required for initial compressed-ZIP support.

## Consequences

### Positive

* Compressed ZIP disc entries can eventually work without unbounded memory.
* CHD remains compressed as CHD rather than expanding into a cached ISO.
* Cache behavior is explicit, bounded, testable, and independent of consumers.
* The existing live decoder and VFS contracts remain unchanged after source
  resolution.
* Users who do not want caching can continue using stored ZIP entries.

### Negative

* A compressed ISO may require cache space equal to its full uncompressed size.
* The first access must wait for complete entry materialization.
* Cache invalidation, concurrency, accounting, and eviction introduce
  operational complexity.
* A source-entry cache creates local persistent or temporary state that must be
  documented and managed.

## Alternatives considered

### Repeatedly decompress from the start for every read

Rejected. Random disc workloads would repeat increasing amounts of work and
could become unusably slow.

### Keep the complete decompressed entry in memory

Rejected. Memory use would scale with disc size and violate bounded-resource
requirements.

### Silently extract every compressed disc entry

Rejected. Disk consumption and startup latency must be explicit and governed by
user-selected policy.

### Implement Deflate seek indexing first

Deferred. It is substantially more complex than disk-backed materialization and
should be justified by measurements after a correct baseline exists.

### Never support compressed disc entries

Rejected as the long-term policy. Compressed ZIP collections are common enough
to warrant a safe compatibility path, even though stored entries remain the
preferred representation for immediate random access.

## Implementation timing

The cache belongs to the performance, caching, and optimization phase. The
current live input-content milestone should retain early rejection for
compressed random-access disc entries and provide repacking guidance.
