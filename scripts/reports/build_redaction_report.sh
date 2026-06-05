#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/build_redaction_report.sh --run-id <run_id> --artifact-root <path> --report-root reports/runs/<run_id>
EOF
  exit 0
fi

echo "build_redaction_report.sh skeleton"
