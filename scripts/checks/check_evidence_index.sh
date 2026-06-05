#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_evidence_index.sh --run-id <run_id> --report-root <path>
EOF
  exit 0
fi

echo "check_evidence_index.sh skeleton"
