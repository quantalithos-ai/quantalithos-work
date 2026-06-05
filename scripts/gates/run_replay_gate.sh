#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/gates/run_replay_gate.sh --run-id <run_id> --artifact-root <path> --config-profile operations-replay --suite <suite> --replay-bundle-ref <ref>
EOF
  exit 0
fi

echo "run_replay_gate.sh skeleton"
