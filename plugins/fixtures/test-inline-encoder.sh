#!/bin/sh
# Retromount fixture plugin:
# - valid protocol v1 manifest
# - high-priority disc encoder
# - returns unmistakable inline bytes ("PLUGIN")
# - intended for integration testing and manual verification only

request="$(cat)"

case "$request" in
  *'"type":"get_manifest"'*)
    cat <<'EOF'
{
  "type": "manifest",
  "manifest": {
    "plugin_id": "plugin.fixture.inline",
    "plugin_version": "1.0.0",
    "protocol_version": {
      "major": 1,
      "minor": 0
    },
    "display_name": "Fixture Inline Encoder",
    "description": "Deterministic fixture plugin for Retromount integration testing",
    "capabilities": [
      {
        "capability_id": "fixture.disc.inline",
        "content_type": "Disc",
        "formats": ["Bin", "Iso", "Chd"],
        "features": [],
        "priority": 1000
      }
    ]
  }
}
EOF
    ;;
  *)
    cat <<'EOF'
{
  "type": "materialized",
  "response": {
    "Inline": {
      "bytes": [80, 76, 85, 71, 73, 78]
    }
  }
}
EOF
    ;;
esac
