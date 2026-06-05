#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/gates/run_integration_like_gate.sh --run-id <run_id> --artifact-root <path> --config-profile integration-like --suite <suite>
EOF
  exit 0
fi

echo "run_integration_like_gate.sh skeleton"
