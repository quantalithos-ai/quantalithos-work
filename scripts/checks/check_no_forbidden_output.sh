#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_no_forbidden_output.sh --run-id <run_id> --artifact-root <path> --report-root <path>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${ARTIFACT_ROOT}" ]] || usage_arg_missing "--artifact-root"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

ensure_dir "${ARTIFACT_ROOT}/redaction-scan"

scan_scope=(
  "${WORK_PROJECT_ROOT}/config"
  "${WORK_PROJECT_ROOT}/reports"
  "${WORK_PROJECT_ROOT}/artifacts"
)

hits_tmp="$(mktemp)"
trap 'rm -f "${hits_tmp}"' EXIT

patterns=(
  "raw secret"
  "raw token"
  "raw payload"
  "source body"
  "ImplementationPlan body"
  "runtime progress body"
)

for pattern in "${patterns[@]}"; do
  rg -n -i --glob '!target/**' --glob '!*Cargo.lock' "${pattern}" "${scan_scope[@]}" >>"${hits_tmp}" || true
done

status="passed"
hit_count=0
if [[ -s "${hits_tmp}" ]]; then
  hit_count="$(wc -l <"${hits_tmp}" | tr -d ' ')"
  status="failed"
fi

sanitized_hits="[]"
if [[ "${status}" == "failed" ]]; then
  sanitized_hits="$(awk -F: '{print "{\"path\":\""$1"\",\"line\":"$2"}"}' "${hits_tmp}" | jq -s '.')"
fi

jq -n \
  --arg run_id "${RUN_ID}" \
  --arg status "${status}" \
  --arg checked_at "$(utc_now)" \
  --argjson hit_count "${hit_count}" \
  --argjson sanitized_hits "${sanitized_hits}" \
  '{
    run_id: $run_id,
    status: $status,
    checked_at: $checked_at,
    hit_count: $hit_count,
    sanitized_hits: $sanitized_hits
  }' >"${ARTIFACT_ROOT}/redaction-scan/report.json"

[[ "${status}" == "passed" ]] || die "forbidden output hit detected"
