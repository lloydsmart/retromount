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
  type: literal_root
  path: DVD

files:
  - select:
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

## Selection rules

Supported `select.type` values:

* `games`
* `games_without_parts`
* `single_disc_games`
* `single_disc_games_by_platform_and_media`
* `multi_disc_games`
* `single_rom_games`
* `bytes`
* `text`

`single_disc_games_by_platform_and_media` also requires `platform` and `media`.
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

## Built-in catalog

The built-in catalog currently contains:

* `flat`
* `grouped`
* `opl`

OPL is loaded from an embedded copy of `presentations/opl.yaml`. Flat and
grouped remain code-constructed declarative values until their shared rule set
is migrated without unnecessary duplication.

## Scope and extension

Schema version 1 intentionally exposes only concepts supported by the current
generic compiler. New fields and rule types should be added in response to a
real presentation requirement, then covered by validation and compilation
tests.
