#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/build_release_summary.sh --run-id <run_id> --report-root reports/runs/<run_id> --acceptance-root reports/acceptance
EOF
  exit 0
fi

echo "build_release_summary.sh skeleton"
