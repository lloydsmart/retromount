#!/bin/sh
# Retromount invalid manifest fixture plugin:
# - executable so discovery inspects it
# - returns an invalid manifest on get_manifest
# - intended for integration testing of plugin discovery/load failures

request="$(cat)"

case "$request" in
*'"type":"get_manifest"'*)
    cat <<'EOF'
{
  "type": "manifest",
  "manifest": {
    "plugin_id": "",
    "plugin_version": "1.0.0",
    "protocol_version": {
      "major": 1,
      "minor": 0
    },
    "display_name": "Fixture Invalid Manifest Encoder",
    "description": "Deterministic invalid-manifest fixture plugin for Retromount integration testing",
    "capabilities": [
      {
        "capability_id": "fixture.invalid",
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
      "bytes": [73, 78, 86, 65, 76, 73, 68]
    }
  }
}
EOF
    ;;
esac
