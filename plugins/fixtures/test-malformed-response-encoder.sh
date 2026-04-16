#!/bin/sh
# Retromount malformed response fixture plugin:
# - valid protocol v1 manifest
# - high-priority disc encoder
# - discovery succeeds and capability selection succeeds
# - materialization returns malformed JSON
# - intended for integration testing of runtime response failures

request="$(cat)"

case "$request" in
*'"type":"get_manifest"'*)
    cat <<'EOF'
{
  "type": "manifest",
  "manifest": {
    "plugin_id": "plugin.fixture.malformed",
    "plugin_version": "1.0.0",
    "protocol_version": {
      "major": 1,
      "minor": 0
    },
    "display_name": "Fixture Malformed Response Encoder",
    "description": "Deterministic malformed-response fixture plugin for Retromount integration testing",
    "capabilities": [
      {
        "capability_id": "fixture.disc.malformed",
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
    printf '%s\n' 'not-json'
    ;;
esac
