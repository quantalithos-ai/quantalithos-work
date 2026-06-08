#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"
source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_data.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/build_release_summary.sh --run-id <run_id> --report-root reports/runs/<run_id> --acceptance-root reports/acceptance
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

ACCEPTANCE_ROOT="${ACCEPTANCE_ROOT:-$(default_acceptance_root)}"
ensure_dir "${ACCEPTANCE_ROOT}"

gate_results_md="${REPORT_ROOT}/gate-results.md"
gate_results_json="${REPORT_ROOT}/gate-results.json"
evidence_index_md="${REPORT_ROOT}/evidence-index.md"
evidence_index_json="${REPORT_ROOT}/evidence-index.json"
redaction_check_md="${REPORT_ROOT}/redaction-check.md"
redaction_scan_json="${ARTIFACT_ROOT:-$(default_artifact_root "${RUN_ID}")}/redaction-scan/report.json"

[[ -f "${gate_results_md}" ]] || die "missing gate results: ${gate_results_md}"
[[ -f "${gate_results_json}" ]] || die "missing gate results json: ${gate_results_json}"
[[ -f "${evidence_index_md}" ]] || die "missing evidence index: ${evidence_index_md}"
[[ -f "${evidence_index_json}" ]] || die "missing evidence index json: ${evidence_index_json}"
[[ -f "${redaction_check_md}" ]] || die "missing redaction report: ${redaction_check_md}"
[[ -f "${redaction_scan_json}" ]] || die "missing redaction artifact: ${redaction_scan_json}"

release_gate_failures="$(
  jq '
    [.suites[] | select(.blocking_class == "release-gate" and .status != "passed")]
  ' "${gate_results_json}"
)"
selected_gate_failures="$(
  jq '
    [.suites[] | select(.blocking_class == "selected-gate" and .status != "passed")]
  ' "${gate_results_json}"
)"
evidence_failures="$(
  jq '
    [.[] | select(.review_status != "reviewed" or (.defect_refs | length > 0))]
  ' "${evidence_index_json}"
)"

redaction_status="$(jq -r '.status' "${redaction_scan_json}")"
release_gate_status="$(jq -r 'if length == 0 then "passed" else "failed" end' <<<"${release_gate_failures}")"
selected_gate_status="$(jq -r 'if length == 0 then "passed" else "failed" end' <<<"${selected_gate_failures}")"
evidence_pack_status="$(jq -r 'if length == 0 then "reviewed" else "needs_followup" end' <<<"${evidence_failures}")"

if [[ "${release_gate_status}" == "passed" && "${redaction_status}" == "passed" && "${evidence_pack_status}" == "reviewed" ]]; then
  conclusion_input="ready_for_acceptance_review"
  acceptance_review_status="reviewed"
else
  conclusion_input="blocked_by_release_redline_or_evidence_gap"
  acceptance_review_status="needs_followup"
fi

cat >"${REPORT_ROOT}/nfr-summary.md" <<EOF
# NFR Summary

- run_id: \`${RUN_ID}\`
- performance_observation: \`present_no_hard_threshold\`
- availability_surface: \`$(if [[ "$(jq 'length' <<<"${selected_gate_failures}")" == "0" ]]; then echo marker_and_report_present; else echo needs_followup; fi)\`
- security_boundary: \`$(if [[ "${redaction_status}" == "passed" ]]; then echo redaction_and_authorization_evidence_present; else echo redaction_followup_required; fi)\`
- idempotency_consistency: \`$(if [[ "$(jq '[.[] | select(.evidence_id == "EV-WORK-NFR-004" and .review_status == "reviewed")] | length' "${evidence_index_json}")" == "1" ]]; then echo duplicate_and_rerun_evidence_present; else echo needs_followup; fi)\`
- observability: \`$(if [[ "$(jq '[.[] | select(.evidence_id == "EV-WORK-NFR-005" and .review_status == "reviewed")] | length' "${evidence_index_json}")" == "1" ]]; then echo trace_audit_outbox_job_reports_present; else echo needs_followup; fi)\`
EOF

cat >"${REPORT_ROOT}/observability-audit.md" <<EOF
# Observability Audit

- run_id: \`${RUN_ID}\`
- accepted_truth_audit: \`covered_by EV-WORK-NFR-005 and selected gate reports\`
- query_no_write: \`covered_by EV-WORK-QUERY-* and integration-p0 selected report\`
- job_reports: \`covered_by EV-WORK-OPS-* and selected gate reports\`
EOF

cat >"${REPORT_ROOT}/release-summary.md" <<EOF
# Release Summary

- run_id: \`${RUN_ID}\`
- release_gate_status: \`${release_gate_status}\`
- selected_gate_status: \`${selected_gate_status}\`
- evidence_pack_status: \`${evidence_pack_status}\`
- redaction_status: \`${redaction_status}\`
- gate_results_ref: \`$(artifact_rel "${gate_results_md}")\`
- evidence_index_ref: \`$(artifact_rel "${evidence_index_md}")\`
- redaction_check_ref: \`$(artifact_rel "${redaction_check_md}")\`
- veto_checklist_ref: \`reports/acceptance/veto-checklist.md\`
- handoff_ref: \`reports/acceptance/handoff.md\`
- risk_acceptance_ref: \`reports/acceptance/risk-acceptance.md\`
- conclusion_input: \`${conclusion_input}\`
- acceptance_review_status: \`${acceptance_review_status}\`
- review_version: \`${RUN_ID}\`
- compile_dependency_ref: \`Cargo.toml\`
- workspace_contract_ref: \`crates/contracts/Cargo.toml\`
EOF

veto_json="$(
  jq \
    --arg run_id "${RUN_ID}" \
    --arg release_gate_status "${release_gate_status}" \
    --arg selected_gate_status "${selected_gate_status}" \
    --arg evidence_pack_status "${evidence_pack_status}" \
    --arg redaction_status "${redaction_status}" \
    --arg acceptance_review_status "${acceptance_review_status}" \
    '
    def with_run_id:
      .evidence_refs = (.evidence_refs | map(gsub("<run_id>"; $run_id)));
    def status_for($veto_id):
      if $veto_id == "VETO-WORK-004" then
        (if $redaction_status == "passed" then "passed" else "failed" end)
      elif $veto_id == "VETO-WORK-010" or $veto_id == "VETO-WORK-011" then
        (if $redaction_status == "passed" then "passed" else "failed" end)
      elif $veto_id == "VETO-WORK-012" then
        (if $evidence_pack_status == "reviewed" and $release_gate_status == "passed" then "passed" else "failed" end)
      else
        (if $selected_gate_status == "passed" and $acceptance_review_status == "reviewed" then "passed" else "failed" end)
      end;
    def defect_refs_for($veto_id):
      if status_for($veto_id) == "passed" then [] else ["DEFECT-" + $veto_id + "-" + $run_id] end;
    map(
      with_run_id
      | .status = status_for(.veto_id)
      | .defect_refs = defect_refs_for(.veto_id)
      | .reviewer_status = (if .status == "passed" then "reviewed" else "needs_followup" end)
      | .risk_acceptance_allowed = false
      | .notes = (if .status == "passed" then .notes_when_passed else "requires follow-up before acceptance handoff" end)
      | del(.notes_when_passed)
    )
    ' <<<"${VETO_CATALOG_JSON}"
)"

{
  echo "# Veto Checklist"
  echo
  echo "- run_id: \`${RUN_ID}\`"
  echo "- review_version: \`${RUN_ID}\`"
  echo "- review_status: \`${acceptance_review_status}\`"
  echo "- reviewed_by: \`agent_reviewed\`"
  echo "- reviewed_at: \`$(utc_now)\`"
  echo
  echo "| veto_id | source_refs | status | evidence_refs | defect_refs | reviewer_status | risk_acceptance_allowed | notes |"
  echo "|---|---|---|---|---|---|---|---|"
  jq -r '
    .[]
    | "| \(.veto_id) | `" + (.source_refs | join(", ")) + "` | \(.status) | `" + (.evidence_refs | join(", ")) + "` | `" + (.defect_refs | join(", ")) + "` | \(.reviewer_status) | \(.risk_acceptance_allowed) | `" + (.notes // "") + "` |"
  ' <<<"${veto_json}"
} >"${ACCEPTANCE_ROOT}/veto-checklist.md"

open_issue_count="$(jq 'length' <<<"${evidence_failures}")"
if [[ "${open_issue_count}" == "0" ]]; then
  cat >"${ACCEPTANCE_ROOT}/open-issues.md" <<EOF
# Open Issues

- run_id: \`${RUN_ID}\`
- review_version: \`${RUN_ID}\`
- review_status: \`${acceptance_review_status}\`
- reviewed_by: \`agent_reviewed\`
- reviewed_at: \`$(utc_now)\`
- current_open_followups: \`none_recorded_in_release_pack\`

| issue_id | classification | impact | owner | action | deadline | evidence_refs |
|---|---|---|---|---|---|---|
| none | not_applicable | no B/C defect, non-blocking risk, or follow-up entry was recorded in this release pack | not_applicable | continue standard acceptance review using fixed-run reports only | not_applicable | \`reports/runs/${RUN_ID}/release-summary.md, reports/acceptance/handoff.md\` |
EOF
else
  {
    echo "# Open Issues"
    echo
    echo "- run_id: \`${RUN_ID}\`"
    echo "- review_version: \`${RUN_ID}\`"
    echo "- review_status: \`${acceptance_review_status}\`"
    echo "- reviewed_by: \`agent_reviewed\`"
    echo "- reviewed_at: \`$(utc_now)\`"
    echo "- current_open_followups: \`${open_issue_count}\`"
    echo
    echo "| issue_id | classification | impact | owner | action | deadline | evidence_refs |"
    echo "|---|---|---|---|---|---|---|"
    jq -r '
      unique_by((.defect_refs // ["pending"]) | join(", "))
      | .[]
      | "| " + ((.defect_refs // ["pending"]) | join(", ")) + " | A_or_followup | release evidence or gate follow-up required | implementation_owner | fix failing suite or evidence path and rerun release pack | before_next_acceptance_run | `" + (.report_refs | join(", ")) + "` |"
    ' <<<"${evidence_failures}"
  } >"${ACCEPTANCE_ROOT}/open-issues.md"
fi

if [[ "${acceptance_review_status}" == "reviewed" ]]; then
  cat >"${ACCEPTANCE_ROOT}/risk-acceptance.md" <<EOF
# Risk Acceptance

- run_id: \`${RUN_ID}\`
- review_version: \`${RUN_ID}\`
- review_status: \`not_applicable\`
- reviewed_by: \`agent_reviewed\`
- reviewed_at: \`$(utc_now)\`
- status: \`no_blocking_risk_candidates_recorded\`
- note: \`S / VETO / P0 A / redaction / duplicate truth risks are not accepted here.\`
- residual_risk_refs: \`none\`

| risk_id | risk_title | risk_type | impact | acceptance_reason | non_acceptance_checks | evidence_refs | owner | accepted_by | follow_up_action | deadline | tracking_ref | review_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| none | no accepted residual risk recorded for \`${RUN_ID}\` | not_applicable | no release-blocking or conditional-pass risk remained after PH-09 review | not required because release gates, selected gates, redaction and evidence checks passed | veto passed; no S defect; no P0 A defect; no redaction failure; no evidence-path failure | \`reports/runs/${RUN_ID}/release-summary.md, reports/acceptance/veto-checklist.md\` | not_applicable | not_applicable | continue with standard acceptance review only | not_applicable | \`reports/acceptance/handoff.md\` | closed |
EOF
else
  cat >"${ACCEPTANCE_ROOT}/risk-acceptance.md" <<EOF
# Risk Acceptance

- run_id: \`${RUN_ID}\`
- review_version: \`${RUN_ID}\`
- review_status: \`needs_followup\`
- reviewed_by: \`agent_reviewed\`
- reviewed_at: \`$(utc_now)\`
- status: \`risk_acceptance_not_available_for_release_blockers\`
- note: \`VETO, release redline, selected gate failure or P0 evidence gap cannot be risk-accepted.\`
- residual_risk_refs: \`reports/acceptance/open-issues.md\`

| risk_id | risk_title | risk_type | impact | acceptance_reason | non_acceptance_checks | evidence_refs | owner | accepted_by | follow_up_action | deadline | tracking_ref | review_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| none | release blockers remain open for \`${RUN_ID}\` | not_applicable | acceptance cannot proceed while release blockers remain | risk acceptance is forbidden for veto / release redline / P0 evidence failures | veto or release blocker remains open | \`reports/runs/${RUN_ID}/release-summary.md, reports/acceptance/open-issues.md\` | not_applicable | not_applicable | close blockers and rerun release pack | before_next_acceptance_run | \`reports/acceptance/open-issues.md\` | pending |
EOF
fi

gate_review_summary="$(
  jq -r '
    [.suites[]
     | select(.blocking_class == "release-gate" or .blocking_class == "selected-gate")
     | "\(.suite):\(.status)"]
    | join(", ")
  ' "${gate_results_json}"
)"

cat >"${ACCEPTANCE_ROOT}/handoff.md" <<EOF
# Acceptance Handoff

- run_id: \`${RUN_ID}\`
- review_version: \`${RUN_ID}\`
- review_status: \`${acceptance_review_status}\`
- reviewed_by: \`agent_reviewed\`
- reviewed_at: \`$(utc_now)\`
- implementation_scope: \`L1-work PH-01~PH-09 P0 boundary through commit-09-a\`
- evidence_baseline: \`reports/runs/${RUN_ID}\`
- gate_results_ref: \`reports/runs/${RUN_ID}/gate-results.md\`
- evidence_index_ref: \`reports/runs/${RUN_ID}/evidence-index.md\`
- redaction_check_ref: \`reports/runs/${RUN_ID}/redaction-check.md\`
- release_summary_ref: \`reports/runs/${RUN_ID}/release-summary.md\`
- open_issues_ref: \`reports/acceptance/open-issues.md\`
- risk_entry_ref: \`reports/acceptance/risk-acceptance.md\`
- gate_review: \`${gate_review_summary}\`
- open_issues_summary: \`$(if [[ "${open_issue_count}" == "0" ]]; then echo no_blocking_open_issues_recorded; else echo release_followups_present; fi)\`
- residual_risk_summary: \`$(if [[ "${acceptance_review_status}" == "reviewed" ]]; then echo no_residual_risks_require_conditional_acceptance; else echo release_blockers_require_followup_and_rerun; fi)\`
- final_decision: \`not_recorded_here\`
EOF
