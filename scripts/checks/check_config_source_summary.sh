#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_config_source_summary.sh --run-id <run_id> --report-root <path>
EOF
  exit 0
fi

echo "check_config_source_summary.sh skeleton"
