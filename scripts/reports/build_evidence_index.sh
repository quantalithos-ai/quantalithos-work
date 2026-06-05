#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/build_evidence_index.sh --run-id <run_id> --report-root reports/runs/<run_id>
EOF
  exit 0
fi

echo "build_evidence_index.sh skeleton"
