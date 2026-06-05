#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_report_paths.sh --run-id <run_id> --artifact-root <path> --report-root <path>
EOF
  exit 0
fi

echo "check_report_paths.sh skeleton"
