#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

readonly CHECK_SCRIPTS_ROOT="${WORK_PROJECT_ROOT}/scripts/checks"
readonly REPORT_SCRIPTS_ROOT="${WORK_PROJECT_ROOT}/scripts/reports"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/gates/run_release_gate.sh --run-id <run_id> --artifact-root <path> --config-profile <profile> --suite <suite>
EOF
  exit 0
fi

require_command cargo
require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${CONFIG_PROFILE}" ]] || usage_arg_missing "--config-profile"
[[ -n "${SUITE}" ]] || usage_arg_missing "--suite"

ARTIFACT_ROOT="${ARTIFACT_ROOT:-$(default_artifact_root "${RUN_ID}")}"
REPORT_ROOT="${REPORT_ROOT:-$(default_report_root "${RUN_ID}")}"

assert_no_latest_path "${ARTIFACT_ROOT}"
assert_no_latest_path "${REPORT_ROOT}"
assert_run_scoped_path "${ARTIFACT_ROOT}" "${RUN_ID}"
assert_run_scoped_path "${REPORT_ROOT}" "${RUN_ID}"

CONFIG_PATH="$(require_config_profile "${CONFIG_PROFILE}")"
SUITE_DIR="$(suite_artifact_dir "${ARTIFACT_ROOT}" "${SUITE}")"

init_suite_layout "${ARTIFACT_ROOT}" "${REPORT_ROOT}" "${SUITE}"
write_context_json "${ARTIFACT_ROOT}" "${RUN_ID}" "${SUITE}" "${CONFIG_PROFILE}" "${CONFIG_PATH}"

run_release_main_smoke() {
  run_cargo_capture \
    "${SUITE_DIR}" \
    "cargo test -q" \
    cargo test -q
}

run_release_config_redline() {
  local stdout_path="${SUITE_DIR}/stdout.log"
  local stderr_path="${SUITE_DIR}/stderr.log"
  local status exit_code
  status="passed"
  exit_code=0

  {
    echo "config_profile=${CONFIG_PROFILE}"
    echo "config_path=$(artifact_rel "${CONFIG_PATH}")"
    echo "report_root=$(artifact_rel "${REPORT_ROOT}")"
    echo "artifact_root=$(artifact_rel "${ARTIFACT_ROOT}")"
  } >"${stdout_path}"
  : >"${stderr_path}"

  set +e
  "${CHECK_SCRIPTS_ROOT}/check_report_paths.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  exit_code=$?
  set -e
  if [[ ${exit_code} -ne 0 ]]; then
    status="failed"
  fi

  set +e
  "${CHECK_SCRIPTS_ROOT}/check_config_source_summary.sh" --run-id "${RUN_ID}" --report-root "${REPORT_ROOT}" --config-profile "${CONFIG_PROFILE}" >>"${stdout_path}" 2>>"${stderr_path}"
  check_exit=$?
  set -e
  if [[ ${check_exit} -ne 0 ]]; then
    status="failed"
    exit_code=${check_exit}
  fi

  set +e
  "${CHECK_SCRIPTS_ROOT}/check_fake_marker.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  check_exit=$?
  set -e
  if [[ ${check_exit} -ne 0 ]]; then
    status="failed"
    exit_code=${check_exit}
  fi

  set +e
  "${CHECK_SCRIPTS_ROOT}/check_no_forbidden_output.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  check_exit=$?
  set -e
  if [[ ${check_exit} -ne 0 ]]; then
    status="failed"
    exit_code=${check_exit}
  fi

  jq -n \
    --arg command_label "release-config-redline checks" \
    --arg status "${status}" \
    --arg started_at "$(utc_now)" \
    --arg ended_at "$(utc_now)" \
    --argjson exit_code "${exit_code}" \
    '{
      command_label: $command_label,
      status: $status,
      started_at: $started_at,
      ended_at: $ended_at,
      exit_code: $exit_code
    }'
}

run_release_evidence_pack() {
  local stdout_path="${SUITE_DIR}/stdout.log"
  local stderr_path="${SUITE_DIR}/stderr.log"
  local status exit_code
  status="passed"
  exit_code=0

  {
    echo "run_id=${RUN_ID}"
    echo "required_reports=gate-results.md,evidence-index.md,redaction-check.md,release-summary.md"
    echo "acceptance_docs=handoff.md,veto-checklist.md,risk-acceptance.md,open-issues.md"
  } >"${stdout_path}"
  : >"${stderr_path}"

  "${REPORT_SCRIPTS_ROOT}/collect_gate_reports.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  "${REPORT_SCRIPTS_ROOT}/build_evidence_index.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  "${REPORT_SCRIPTS_ROOT}/build_redaction_report.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  "${REPORT_SCRIPTS_ROOT}/build_release_summary.sh" --run-id "${RUN_ID}" --report-root "${REPORT_ROOT}" --acceptance-root "$(default_acceptance_root)" >>"${stdout_path}" 2>>"${stderr_path}"

  set +e
  "${CHECK_SCRIPTS_ROOT}/check_evidence_index.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}" >>"${stdout_path}" 2>>"${stderr_path}"
  exit_code=$?
  set -e
  if [[ ${exit_code} -ne 0 ]]; then
    status="failed"
  fi

  jq -n \
    --arg command_label "release-evidence-pack assembly" \
    --arg status "${status}" \
    --arg started_at "$(utc_now)" \
    --arg ended_at "$(utc_now)" \
    --argjson exit_code "${exit_code}" \
    '{
      command_label: $command_label,
      status: $status,
      started_at: $started_at,
      ended_at: $ended_at,
      exit_code: $exit_code
    }'
}

case "${SUITE}" in
  release-main-smoke)
    SUITE_RESULT_JSON="$(run_release_main_smoke)"
    ;;
  release-config-redline)
    SUITE_RESULT_JSON="$(run_release_config_redline)"
    ;;
  release-evidence-pack)
    SUITE_RESULT_JSON="$(run_release_evidence_pack)"
    ;;
  *)
    die "unsupported release suite: ${SUITE}"
    ;;
esac

SUITE_STATUS="$(printf '%s\n' "${SUITE_RESULT_JSON}" | jq -r '.status')"
EXIT_CODE="$(printf '%s\n' "${SUITE_RESULT_JSON}" | jq -r '.exit_code')"

write_json "${SUITE_DIR}/report.json" "${SUITE_RESULT_JSON}"
if [[ "${SUITE_STATUS}" == "passed" ]]; then
  write_failure_reason "${SUITE_DIR}/failure-reason.json" "${SUITE}" "not_applicable"
else
  write_failure_reason "${SUITE_DIR}/failure-reason.json" "${SUITE}" "release suite failed"
fi

finalize_suite_report \
  "${ARTIFACT_ROOT}" \
  "${REPORT_ROOT}" \
  "${RUN_ID}" \
  "${SUITE}" \
  "${CONFIG_PROFILE}" \
  "${SUITE}"

if [[ "${SUITE}" == "release-evidence-pack" ]]; then
  "${REPORT_SCRIPTS_ROOT}/collect_gate_reports.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}"
  "${REPORT_SCRIPTS_ROOT}/build_evidence_index.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}"
  "${REPORT_SCRIPTS_ROOT}/build_redaction_report.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}"
  "${REPORT_SCRIPTS_ROOT}/build_release_summary.sh" --run-id "${RUN_ID}" --report-root "${REPORT_ROOT}" --acceptance-root "$(default_acceptance_root)"
  "${CHECK_SCRIPTS_ROOT}/check_evidence_index.sh" --run-id "${RUN_ID}" --artifact-root "${ARTIFACT_ROOT}" --report-root "${REPORT_ROOT}"
fi

[[ "${EXIT_CODE}" == "0" ]] || exit "${EXIT_CODE}"
