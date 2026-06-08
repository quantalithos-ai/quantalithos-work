#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_report_paths.sh --run-id <run_id> --artifact-root <path> --report-root <path>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${ARTIFACT_ROOT}" ]] || usage_arg_missing "--artifact-root"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

assert_no_latest_path "${ARTIFACT_ROOT}"
assert_no_latest_path "${REPORT_ROOT}"
assert_run_scoped_path "${ARTIFACT_ROOT}" "${RUN_ID}"
assert_run_scoped_path "${REPORT_ROOT}" "${RUN_ID}"

local_acceptance_root="$(default_acceptance_root)"
assert_no_latest_path "${local_acceptance_root}"

ensure_dir "${ARTIFACT_ROOT}/checks"

artifact_rel_root="$(artifact_rel "${ARTIFACT_ROOT}")"
report_rel_root="$(artifact_rel "${REPORT_ROOT}")"
acceptance_rel_root="$(artifact_rel "${local_acceptance_root}")"

status="passed"
reason="paths are run-scoped and do not reference latest"
if [[ "${artifact_rel_root}" != "artifacts/test/${RUN_ID}" ]]; then
  status="failed"
  reason="artifact root must be artifacts/test/<run_id>"
fi
if [[ "${report_rel_root}" != "reports/runs/${RUN_ID}" ]]; then
  status="failed"
  reason="report root must be reports/runs/<run_id>"
fi
if [[ "${acceptance_rel_root}" != "reports/acceptance" ]]; then
  status="failed"
  reason="acceptance root must be reports/acceptance"
fi

jq -n \
  --arg run_id "${RUN_ID}" \
  --arg artifact_root "${artifact_rel_root}" \
  --arg report_root "${report_rel_root}" \
  --arg acceptance_root "${acceptance_rel_root}" \
  --arg status "${status}" \
  --arg reason "${reason}" \
  --arg checked_at "$(utc_now)" \
  '{
    run_id: $run_id,
    artifact_root: $artifact_root,
    report_root: $report_root,
    acceptance_root: $acceptance_root,
    status: $status,
    reason: $reason,
    checked_at: $checked_at
  }' >"${ARTIFACT_ROOT}/checks/report-paths.json"

[[ "${status}" == "passed" ]] || die "${reason}"
