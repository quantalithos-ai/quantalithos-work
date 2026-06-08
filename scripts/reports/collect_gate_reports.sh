#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/collect_gate_reports.sh --run-id <run_id> --artifact-root <path> --report-root reports/runs/<run_id>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${ARTIFACT_ROOT}" ]] || usage_arg_missing "--artifact-root"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

ensure_dir "${REPORT_ROOT}"
ensure_dir "${REPORT_ROOT}/suites"

gate_results_json='[]'

suite_blocking_class() {
  local suite="$1"
  case "${suite}" in
    release-main-smoke|release-config-redline|release-evidence-pack)
      echo "release-gate"
      ;;
    service-all|integration-p0|worker-job-contract|consumer-outbox|config-redaction|operations-replay|integration-like-seam)
      echo "selected-gate"
      ;;
    *)
      echo "supporting"
      ;;
  esac
}

while IFS= read -r report_json; do
  suite="$(basename "$(dirname "${report_json}")")"
  status="$(jq -r '.status' "${report_json}")"
  exit_code="$(jq -r '.exit_code' "${report_json}")"
  blocking_class="$(suite_blocking_class "${suite}")"
  if [[ "${status}" == "passed" ]]; then
    defect_refs='[]'
  else
    defect_refs="$(
      jq -cn \
        --arg suite "${suite}" \
        --arg run_id "${RUN_ID}" \
        '["DEFECT-" + ($suite | ascii_upcase | gsub("[^A-Z0-9]+"; "-")) + "-" + $run_id]'
    )"
  fi
  failure_rel="$(artifact_rel "$(dirname "${report_json}")/failure-reason.json")"
  suite_report="$(suite_report_path "${REPORT_ROOT}" "${suite}")"
  suite_root_report="${REPORT_ROOT}/${suite}.md"

  if [[ ! -f "${suite_report}" ]]; then
    write_suite_markdown \
      "${suite_report}" \
      "${suite}" \
      "${status}" \
      "${RUN_ID}" \
      "${suite}" \
      "ci-test" \
      "$(artifact_rel "${report_json}")" \
      "$(artifact_rel "$(dirname "${report_json}")/stdout.log")" \
      "$(artifact_rel "$(dirname "${report_json}")/stderr.log")" \
      "${failure_rel}"
  fi
  cp "${suite_report}" "${suite_root_report}"

  gate_results_json="$(
    jq -c \
      --arg suite "${suite}" \
      --arg status "${status}" \
      --arg blocking_class "${blocking_class}" \
      --arg reason "$(jq -r '.reason' "$(dirname "${report_json}")/failure-reason.json")" \
      --arg report_ref "$(artifact_rel "${suite_root_report}")" \
      --arg artifact_ref "$(artifact_rel "${report_json}")" \
      --argjson exit_code "${exit_code}" \
      --argjson defect_refs "${defect_refs}" \
      '. + [{
        suite: $suite,
        status: $status,
        blocking_class: $blocking_class,
        reason: $reason,
        report_ref: $report_ref,
        artifact_ref: $artifact_ref,
        exit_code: $exit_code,
        defect_refs: $defect_refs
      }]' <<<"${gate_results_json}"
  )"
done < <(find "${ARTIFACT_ROOT}/suites" -mindepth 2 -maxdepth 2 -name report.json | sort)

gate_results_md="${REPORT_ROOT}/gate-results.md"
{
  echo "# Gate Results"
  echo
  echo "- run_id: \`${RUN_ID}\`"
  echo
  echo "| suite | blocking_class | status | exit_code | reason | defect_refs | report_ref | artifact_ref |"
  echo "|---|---|---|---:|---|---|---|---|"
  jq -r '.[] | "| \(.suite) | \(.blocking_class) | \(.status) | \(.exit_code) | \(.reason) | `" + (.defect_refs | join(", ")) + "` | `\(.report_ref)` | `\(.artifact_ref)` |"' <<<"${gate_results_json}"
} >"${gate_results_md}"

jq -n \
  --arg run_id "${RUN_ID}" \
  --arg generated_at "$(utc_now)" \
  --argjson suites "${gate_results_json}" \
  '{
    run_id: $run_id,
    generated_at: $generated_at,
    suites: $suites
  }' >"${REPORT_ROOT}/gate-results.json"
