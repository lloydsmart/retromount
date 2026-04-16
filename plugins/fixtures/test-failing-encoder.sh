#!/bin/sh
# Retromount failing fixture plugin:
# - valid protocol v1 manifest
# - high-priority disc encoder
# - discovery succeeds
# - materialization fails deterministically
# - intended for integration testing of runtime failure handling

request="$(cat)"

case "$request" in
*'"type":"get_manifest"'*)
    cat <<'EOF'
{
  "type": "manifest",
  "manifest": {
    "plugin_id": "plugin.fixture.failing",
    "plugin_version": "1.0.0",
    "protocol_version": {
      "major": 1,
      "minor": 0
    },
    "display_name": "Fixture Failing Encoder",
    "description": "Deterministic failing fixture plugin for Retromount integration testing",
    "capabilities": [
      {
        "capability_id": "fixture.disc.failing",
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
    echo "fixture plugin forced materialization failure" >&2
    exit 42
    ;;
esac
