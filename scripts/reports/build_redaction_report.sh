#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/build_redaction_report.sh --run-id <run_id> --artifact-root <path> --report-root reports/runs/<run_id>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${ARTIFACT_ROOT}" ]] || usage_arg_missing "--artifact-root"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

redaction_json="${ARTIFACT_ROOT}/redaction-scan/report.json"
path_json="${ARTIFACT_ROOT}/checks/report-paths.json"
fake_json="${ARTIFACT_ROOT}/checks/fake-marker.json"

[[ -f "${redaction_json}" ]] || die "missing redaction artifact: ${redaction_json}"
[[ -f "${path_json}" ]] || die "missing path check artifact: ${path_json}"
[[ -f "${fake_json}" ]] || die "missing fake marker artifact: ${fake_json}"

status="passed"
if [[ "$(jq -r '.status' "${redaction_json}")" != "passed" ]]; then
  status="failed"
fi
if [[ "$(jq -r '.status' "${path_json}")" != "passed" ]]; then
  status="failed"
fi
if [[ "$(jq -r '.status' "${fake_json}")" != "passed" ]]; then
  status="failed"
fi

cat >"${REPORT_ROOT}/redaction-check.md" <<EOF
# Redaction Check

- run_id: \`${RUN_ID}\`
- status: \`${status}\`
- redaction_artifact: \`$(artifact_rel "${redaction_json}")\`
- path_check_artifact: \`$(artifact_rel "${path_json}")\`
- fake_marker_artifact: \`$(artifact_rel "${fake_json}")\`
- hit_count: \`$(jq -r '.hit_count' "${redaction_json}")\`
EOF

[[ "${status}" == "passed" ]] || die "redaction checks failed"
