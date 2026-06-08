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

write_suite_batch_result() {
  local suite_name="$1"
  local command_label="$2"
  local started_at="$3"
  local status="$4"
  local exit_code="$5"
  local steps_json="$6"
  local suite_dir
  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "${suite_name}")"
  ensure_dir "${suite_dir}"
  write_json "${suite_dir}/command.json" "${steps_json}"
  jq -n \
    --arg command_label "${command_label}" \
    --arg status "${status}" \
    --arg started_at "${started_at}" \
    --arg ended_at "$(utc_now)" \
    --argjson exit_code "${exit_code}" \
    --argjson steps "${steps_json}" \
    '{
      command_label: $command_label,
      status: $status,
      started_at: $started_at,
      ended_at: $ended_at,
      exit_code: $exit_code,
      steps: $steps
    }' >"${suite_dir}/report.json"
  if [[ "${status}" == "passed" ]]; then
    write_failure_reason "${suite_dir}/failure-reason.json" "${suite_name}" "not_applicable"
  else
    write_failure_reason "${suite_dir}/failure-reason.json" "${suite_name}" "selected release suite failed"
  fi

  finalize_suite_report \
    "${ARTIFACT_ROOT}" \
    "${REPORT_ROOT}" \
    "${RUN_ID}" \
    "${suite_name}" \
    "${CONFIG_PROFILE}" \
    "${suite_name}"
}

run_suite_test_batch() {
  local suite_name="$1"
  local command_label="$2"
  shift 2

  local suite_dir stdout_path stderr_path started_at status exit_code
  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "${suite_name}")"
  ensure_dir "${suite_dir}"
  stdout_path="${suite_dir}/stdout.log"
  stderr_path="${suite_dir}/stderr.log"
  started_at="$(utc_now)"
  status="passed"
  exit_code=0
  : >"${stdout_path}"
  : >"${stderr_path}"

  local steps_json='[]'
  local entry package_name target_kind target_name test_name
  for entry in "$@"; do
    IFS='|' read -r package_name target_kind target_name test_name <<<"${entry}"
    local command=(cargo test -q -p "${package_name}")
    if [[ "${target_kind}" == "integration-test" ]]; then
      command+=(--test "${target_name}")
    fi
    command+=("${test_name}" -- --exact)

    printf 'test=%s command=%s\n' "${test_name}" "${command[*]}" >>"${stdout_path}"
    set +e
    "${command[@]}" >>"${stdout_path}" 2>>"${stderr_path}"
    local step_exit_code=$?
    set -e
    steps_json="$(
      jq -c \
        --arg package_name "${package_name}" \
        --arg target_kind "${target_kind}" \
        --arg target_name "${target_name}" \
        --arg test_name "${test_name}" \
        --arg command "${command[*]}" \
        --arg status "$(if [[ ${step_exit_code} -eq 0 ]]; then echo passed; else echo failed; fi)" \
        --argjson exit_code "${step_exit_code}" \
        '. + [{
          package_name: $package_name,
          target_kind: $target_kind,
          target_name: $target_name,
          test_name: $test_name,
          command: $command,
          status: $status,
          exit_code: $exit_code
        }]' <<<"${steps_json}"
    )"
    if [[ ${step_exit_code} -ne 0 ]]; then
      status="failed"
      exit_code=${step_exit_code}
      break
    fi
  done

  normalize_text_file_eof "${stdout_path}"
  normalize_text_file_eof "${stderr_path}"
  write_suite_batch_result "${suite_name}" "${command_label}" "${started_at}" "${status}" "${exit_code}" "${steps_json}"
}

run_config_redline_suite() {
  local suite_name="$1"
  local suite_dir stdout_path stderr_path started_at status exit_code check_exit steps_json
  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "${suite_name}")"
  ensure_dir "${suite_dir}"
  stdout_path="${suite_dir}/stdout.log"
  stderr_path="${suite_dir}/stderr.log"
  started_at="$(utc_now)"
  status="passed"
  exit_code=0
  steps_json='[]'

  {
    echo "config_profile=${CONFIG_PROFILE}"
    echo "config_path=$(artifact_rel "${CONFIG_PATH}")"
    echo "report_root=$(artifact_rel "${REPORT_ROOT}")"
    echo "artifact_root=$(artifact_rel "${ARTIFACT_ROOT}")"
  } >"${stdout_path}"
  : >"${stderr_path}"

  local -a commands=(
    "${CHECK_SCRIPTS_ROOT}/check_report_paths.sh --run-id ${RUN_ID} --artifact-root ${ARTIFACT_ROOT} --report-root ${REPORT_ROOT}"
    "${CHECK_SCRIPTS_ROOT}/check_config_source_summary.sh --run-id ${RUN_ID} --report-root ${REPORT_ROOT} --config-profile ${CONFIG_PROFILE}"
    "${CHECK_SCRIPTS_ROOT}/check_fake_marker.sh --run-id ${RUN_ID} --artifact-root ${ARTIFACT_ROOT} --report-root ${REPORT_ROOT}"
    "${CHECK_SCRIPTS_ROOT}/check_no_forbidden_output.sh --run-id ${RUN_ID} --artifact-root ${ARTIFACT_ROOT} --report-root ${REPORT_ROOT}"
  )

  local command
  for command in "${commands[@]}"; do
    printf 'command=%s\n' "${command}" >>"${stdout_path}"
    set +e
    /usr/bin/env bash -lc "${command}" >>"${stdout_path}" 2>>"${stderr_path}"
    check_exit=$?
    set -e
    steps_json="$(
      jq -c \
        --arg command "${command}" \
        --arg status "$(if [[ ${check_exit} -eq 0 ]]; then echo passed; else echo failed; fi)" \
        --argjson exit_code "${check_exit}" \
        '. + [{
          command: $command,
          status: $status,
          exit_code: $exit_code
        }]' <<<"${steps_json}"
    )"
    if [[ ${check_exit} -ne 0 ]]; then
      status="failed"
      exit_code=${check_exit}
      break
    fi
  done

  normalize_text_file_eof "${stdout_path}"
  normalize_text_file_eof "${stderr_path}"
  write_suite_batch_result "${suite_name}" "${suite_name} checks" "${started_at}" "${status}" "${exit_code}" "${steps_json}"
}

run_release_main_smoke() {
  run_suite_test_batch \
    "release-main-smoke" \
    "release-main-smoke selected cases" \
    "work-api|unit-test|src/lib.rs|tests::tc_work_core_001_create_project_persists_project_backlog_and_side_effects" \
    "work-api|unit-test|src/lib.rs|tests::tc_work_member_001_assign_project_member_persists_member_snapshot_and_side_effects" \
    "work-api|unit-test|src/lib.rs|tests::tc_work_formal_001_create_work_item_persists_truth_membership_and_side_effects" \
    "work-api|unit-test|src/lib.rs|tests::tc_work_iter_001_open_iteration_validates_process_timebox_summary_and_duplicate" \
    "work-api|unit-test|src/lib.rs|tests::tc_work_query_007_get_work_trace_page_empty_not_visible" \
    "work-api|unit-test|src/lib.rs|tests::tc_work_promote_001_request_promotion_persists_result_trace_and_outbox" \
    "work-jobs|integration-test|outbox|runner_delegates_publish_work_outbox" \
    "work-application|integration-test|outbox|publish_outbox_success_marks_published_and_saves_job_report" \
    "work-jobs|integration-test|ops|refresh_job_marks_repository_returned_affected_views_stale"

  cat "${SUITE_DIR}/report.json"
}

run_release_config_redline() {
  run_config_redline_suite "release-config-redline"
  cat "${SUITE_DIR}/report.json"
}

run_selected_release_suites_if_missing() {
  local suite_dir

  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "service-all")"
  [[ -f "${suite_dir}/report.json" ]] || \
    run_suite_test_batch \
      "service-all" \
      "service-all selected release cases" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_core_001_create_project_persists_project_backlog_and_side_effects" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_core_002_missing_project_write_does_not_implicitly_create_truth" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_core_003_update_project_lifecycle_archives_backlog" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_core_004_create_project_duplicate_replays_stored_result" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_member_001_assign_project_member_persists_member_snapshot_and_side_effects" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_member_002_identity_resolver_unresolved_or_unavailable_does_not_save_truth" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_member_003_body_leak_rejects_identity_truth_takeover" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_member_004_released_member_cannot_return_to_active" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_formal_001_create_work_item_persists_truth_membership_and_side_effects" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_formal_002_external_body_rejected_without_work_truth_write" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_formal_003_locked_backlog_rejects_new_work_item" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_formal_005_child_create_and_invalid_parent_lifecycle_completion" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_promote_001_request_promotion_persists_result_trace_and_outbox" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_promote_002_review_accept_creates_formal_work" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_promote_003_review_reject_records_decision_without_work" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_promote_005_review_version_conflict_has_single_winner" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_dep_001_link_work_dependency_persists_truth_and_side_effects" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_dep_002_cycle_reject_does_not_write_truth" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_dep_003_update_dependency_state_requires_reason_and_evidence" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_dep_004_open_blocker_persists_truth_and_side_effects" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_dep_005_resolve_blocker_requires_verified_evidence" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_iter_001_open_iteration_validates_process_timebox_summary_and_duplicate" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_iter_002_commit_scope_marks_root_and_child_work_committed" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_iter_003_update_commitment_rejects_non_member_work_and_updates_state" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_iter_004_lifecycle_validates_reason_shape_and_closes_commitment" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_iter_005_start_requires_existing_commitment"

  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "integration-p0")"
  [[ -f "${suite_dir}/report.json" ]] || \
    run_suite_test_batch \
      "integration-p0" \
      "integration-p0 selected release cases" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_001_project_work_facts_hit_missing_not_visible" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_002_backlog_page_and_empty" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_003_work_item_visible_and_not_visible" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_004_member_work_projection_surfaces" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_005_iteration_summary_projection_surface" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_006_search_work_criteria_failed_and_no_write" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_007_get_work_trace_page_empty_not_visible" \
      "work-api|unit-test|src/lib.rs|tests::tc_work_query_008_get_project_board_view_board_and_rebuilding"

  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "worker-job-contract")"
  [[ -f "${suite_dir}/report.json" ]] || \
    run_suite_test_batch \
      "worker-job-contract" \
      "worker-job-contract selected release cases" \
      "work-contracts|unit-test|src/lib.rs|tests::event_schema_and_job_contracts_roundtrip" \
      "work-jobs|integration-test|outbox|runner_delegates_publish_work_outbox" \
      "work-worker|unit-test|src/lib.rs|tests::unsupported_method_event_version_dead_letters_before_write"

  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "consumer-outbox")"
  [[ -f "${suite_dir}/report.json" ]] || \
    run_suite_test_batch \
      "consumer-outbox" \
      "consumer-outbox selected release cases" \
      "work-application|integration-test|consumer|consume_identity_member_changed_marks_repository_returned_views_stale" \
      "work-application|integration-test|consumer|duplicate_identity_event_does_not_repeat_snapshot_or_stale_marker" \
      "work-application|integration-test|outbox|publish_outbox_success_marks_published_and_saves_job_report" \
      "work-application|integration-test|outbox|publish_outbox_invalid_source_marks_failed_without_partial_publish" \
      "work-application|integration-test|outbox|duplicate_job_replays_stored_report_without_rescanning_or_republishing"

  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "operations-replay")"
  [[ -f "${suite_dir}/report.json" ]] || \
    run_suite_test_batch \
      "operations-replay" \
      "operations-replay selected release cases" \
      "work-jobs|integration-test|ops|rebuild_job_replaces_projection_from_committed_truth" \
      "work-jobs|integration-test|ops|refresh_job_marks_repository_returned_affected_views_stale" \
      "work-jobs|integration-test|ops|reconciliation_job_is_read_only" \
      "work-jobs|integration-test|ops|trace_handoff_job_saves_marker_and_replays_duplicate" \
      "work-jobs|integration-test|ops|archive_handoff_job_reports_typed_failure_and_replays_duplicate"

  suite_dir="$(suite_artifact_dir "${ARTIFACT_ROOT}" "config-redaction")"
  [[ -f "${suite_dir}/report.json" ]] || run_config_redline_suite "config-redaction"
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

  run_selected_release_suites_if_missing >>"${stdout_path}" 2>>"${stderr_path}"

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
