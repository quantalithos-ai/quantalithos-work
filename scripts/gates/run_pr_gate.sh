#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/gates/run_pr_gate.sh --run-id <run_id> --artifact-root <path> --config-profile <profile> --suite <suite>
EOF
  exit 0
fi

echo "run_pr_gate.sh skeleton"
