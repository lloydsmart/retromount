# ZIP entry names

Retromount identifies ZIP members by their exact decoded names in source
references of the form `zip:<archive>#<entry-name>`. CP437 and UTF-8 names use
the same read paths. Matching is case-sensitive and does not apply Unicode
normalisation.

## Ambiguity policy

Distinct raw names can decode to the same Unicode name. Retromount rejects
these collisions with `io::ErrorKind::InvalidData` and an error identifying
the ambiguous name. Rename the conflicting members before loading the archive.

Enumeration rejects duplicate decoded file names before returning source
objects. Direct reads and source-reference resolution require one exact decoded
match; compressed reads repeat this check when reopening the archive. A missing
decoded name returns `NotFound`.

We deliberately do not prefer UTF-8 members or use archive order to break ties.
Those choices would allow two enumerated members to share one source identity
while opening only one payload. Raw-byte lookup alone also cannot establish decoded-name uniqueness.
The ZIP library recognises valid UTF-8 bytes even when the UTF-8 flag is clear,
so unflagged UTF-8 members participate in this check as well.

The resolver scans central-directory names without opening or decompressing
payloads. This is O(number of entries) per lookup, including compressed-entry
reopening. An unconditional raw-name fast path cannot establish uniqueness.
A future validated index could avoid repeated scans, but must preserve the
same matching and ambiguity rules.

This policy addresses distinct raw names decoding to the same name. It does
not add general archive validation or change how the ZIP library handles
duplicate identical raw-name records. It also does not make open readers
stable snapshots of an archive that is replaced or modified.

## Regression coverage

`tests/zip_cp437_integration.rs` constructs ZIP headers independently of
`ZipWriter`, with explicit raw filename bytes, encoding flags and Stored or
Deflate methods. It covers:

- CP437 enumeration, source identities, content opening, direct readers and
  source-reference opening/resolution, with ASCII and UTF-8 controls.
- Repeated, out-of-order reads and reads after EOF for compressed reopening.
- CUE-relative BIN lookup inside a CP437 directory, live BIN/logical reads,
  and matching, validation and byte-exact reading of an SBI sidecar.
- Duplicate decoded names in either member order and both compression modes.
- Valid unflagged UTF-8 names, both alone and colliding with CP437 names.
- Collision detection when a compressed entry is reopened, and missing-name,
  case-sensitive and Unicode-normalisation controls.

CUE and SBI entries are tested with both compression methods. BIN entries are
stored because the disc decoder requires efficient random access; compressed
BIN rejection remains covered by the existing integration suite.
