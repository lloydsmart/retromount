# Presentation Files

Retromount presentations describe filesystem layout, content selection, naming,
and output artifact requirements as versioned YAML data. The generic
presentation compiler turns this data into a `PresentationPlan`; encoders then
materialize the requested artifacts.

Presentation files do not contain executable logic and do not select input
formats or named encoders.

## Selecting a presentation

Use a built-in presentation name:

```bash
retromount inspect /roms --presentation grouped
retromount mount /roms/ps2/Game.chd /mnt/retromount --presentation opl
```

Or provide a YAML file:

```bash
retromount inspect /roms --presentation ./my-presentation.yaml
retromount mount /roms /mnt/retromount \
  --presentation ./my-presentation.yaml
```

The older `--view` flag remains an alias for compatibility.

Configured views use the same name-or-path value:

```yaml
- name: custom
  source: /roms
  mount: /mnt/retromount
  platform: ps2
  presentation: ./my-presentation.yaml
```

The older `presenter` field remains supported as a legacy alias. A view must not
define both fields.

## Schema version 1

Every file declares its schema version and presentation name:

```yaml
version: 1
name: opl
```

Unknown versions and unknown fields are rejected. This prevents misspellings
from silently changing output.

### Complete example

```yaml
version: 1
name: opl

layout:
  type: flat

files:
  - directory: DVD
    select:
      type: single_disc_games_by_platform_and_media
      platform: ps2
      media: dvd
    naming:
      type: game_title
    artifact:
      content_type: disc
      format: iso
      required_features:
        - random_access
        - lossless
```

The repository version of this presentation is
[`presentations/opl.yaml`](../presentations/opl.yaml).

## Layouts

Supported `layout.type` values:

* `flat`
* `grouped_by_platform_and_game`
* `literal_root`, with a non-empty `path`

## Rule destination directories

A file rule may declare an optional relative virtual `directory`. The compiler
creates that directory below the selected layout location:

```yaml
files:
  - directory: CD
    select:
      type: single_disc_games_by_platform_and_media
      platform: ps2
      media: cd
    naming:
      type: game_title
    artifact:
      content_type: disc
      format: iso
```

Nested directories use `/` separators. Absolute paths, drive prefixes, empty
segments, `.`, and `..` segments are rejected. Backslashes are normalized as
virtual separators when a presentation is loaded.

## Selection rules

Supported `select.type` values:

* `games`
* `games_without_parts`
* `single_disc_games`
* `single_disc_games_by_platform`
* `single_disc_games_by_platform_and_media`
* `multi_disc_games`
* `single_rom_games`
* `bytes`
* `text`

`single_disc_games_by_platform` requires `platform`.
`single_disc_games_by_platform_and_media` requires `platform` and `media`.
Schema version 1 supports the platforms and media kinds already present in the
normalized content model.

## Naming rules

Supported `naming.type` values:

* `game_title`
* `game_name`
* `part_name`
* `playlist_name`
* `source_name`
* `literal`, with a non-empty `value`

## Artifact requirements

Each rule declares a `content_type` and may declare:

* `format`
* `required_features`
* `preferred_features`
* `forbidden_features`

A feature cannot be required or preferred while also forbidden. Capability
resolution selects an encoder after the presentation has compiled.

`format: cue_bin` requests one atomic multi-file disc artifact. Its encoder
produces a generated CUE and live per-track BIN files inside one game
directory. The `multi_file` feature describes that one-to-many output and is
distinct from `multi_source`, which describes multiple encoder inputs.

## Built-in catalog

The built-in catalog currently contains:

* `flat`
* `grouped`
* `duckstation`
* `opl`

DuckStation and OPL are loaded from embedded copies of
`presentations/duckstation.yaml` and `presentations/opl.yaml`. Flat and grouped
remain code-constructed declarative values until their shared rule set is
migrated without unnecessary duplication.

The OPL presentation declares sibling `DVD/` and `CD/` rules. ISO inputs need
an explicit `media: dvd` or `media: cd` composition hint because ISO does not
reliably identify its original carrier. Track-aware CUE/BIN inputs provide CD
media semantics directly, but mixed-mode, audio, and other layouts without a
safe 2048-byte logical projection are rejected by the OPL composition.

The `duckstation` contract is defined by
[ADR-013](architecture/adr-013-ps1-duckstation-presentation-contract.md).
Unlike OPL, it consumes the complete track-aware PS1 CD and produces one
coherent artifact set containing either a native CHD or a generated CUE with
live per-track BIN files.
It supports single-disc CUE/BIN, CHD, and cooked data-only ISO input. Existing
CHDs are presented byte-for-byte with an optional same-stem SBI, preserving
embedded subchannel data without re-encoding. Cooked ISO is represented
honestly as one `MODE1/2048` track; Retromount does not synthesize absent
raw-sector headers, audio, pregaps, or subchannel data. CUE/BIN and ISO inputs
continue to produce generated CUE and live track BIN files.

Multi-disc PS1 games are allocated as one game directory containing an ordered
artifact set for each disc and a sibling M3U playlist. Existing CHDs remain
native; other supported inputs produce CUE/BIN. Playlist entries use portable
relative paths to the selected disc artifacts, so conflict-renamed game
directories remain internally coherent.

The PS1 roadmap also commits to extending the existing `opl` presentation with
POPS support. The resulting PS2 library view will retain PS2 games below
`DVD/` and `CD/` and add PS1 VCD content below `POPS/`; there will not be a
standalone POPS presentation. VCD conversion is still a separate encoder
capability and does not reuse DuckStation's CUE/BIN artifact contract. The
exact OPL, POPStarter, or POPSLoader compatibility target and storage backend
require a dedicated research ADR before the new OPL rule is fixed.

## Scope and extension

Schema version 1 intentionally exposes only concepts supported by the current
generic compiler. New fields and rule types should be added in response to a
real presentation requirement, then covered by validation and compilation
tests.

Rules may constrain `source_formats` or `excluded_source_formats`. These
filters select representations from the normalized source identity; they do
not reinterpret or convert the source. A format cannot appear in both sets on
one rule, and every part of a multi-part game must satisfy the selected rule.
