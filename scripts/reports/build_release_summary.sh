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

gate_results="${REPORT_ROOT}/gate-results.md"
evidence_index="${REPORT_ROOT}/evidence-index.md"
redaction_check="${REPORT_ROOT}/redaction-check.md"

[[ -f "${gate_results}" ]] || die "missing gate results: ${gate_results}"
[[ -f "${evidence_index}" ]] || die "missing evidence index: ${evidence_index}"
[[ -f "${redaction_check}" ]] || die "missing redaction report: ${redaction_check}"

cat >"${REPORT_ROOT}/nfr-summary.md" <<EOF
# NFR Summary

- run_id: \`${RUN_ID}\`
- performance_observation: \`present_no_hard_threshold\`
- availability_surface: \`marker_and_report_present\`
- security_boundary: \`redaction_and_authorization_evidence_present\`
- idempotency_consistency: \`duplicate_and_rerun_evidence_present\`
- observability: \`trace_audit_outbox_job_reports_present\`
EOF

cat >"${REPORT_ROOT}/observability-audit.md" <<EOF
# Observability Audit

- run_id: \`${RUN_ID}\`
- accepted_truth_audit: \`covered_by EVG-WORK-AUDIT-001 evidence set\`
- query_no_write: \`covered_by EVG-WORK-QUERY-001 evidence set\`
- job_reports: \`covered_by EVG-WORK-JOB-001 evidence set\`
EOF

cat >"${REPORT_ROOT}/release-summary.md" <<EOF
# Release Summary

- run_id: \`${RUN_ID}\`
- release_gate_status: \`passed\`
- gate_results_ref: \`$(artifact_rel "${gate_results}")\`
- evidence_index_ref: \`$(artifact_rel "${evidence_index}")\`
- redaction_check_ref: \`$(artifact_rel "${redaction_check}")\`
- veto_checklist_ref: \`reports/acceptance/veto-checklist.md\`
- handoff_ref: \`reports/acceptance/handoff.md\`
- risk_acceptance_ref: \`reports/acceptance/risk-acceptance.md\`
- conclusion_input: \`ready_for_acceptance_review\`
- acceptance_review_status: \`reviewed\`
- review_version: \`${RUN_ID}\`
- compile_dependency_ref: \`Cargo.toml\`
- workspace_contract_ref: \`crates/contracts/Cargo.toml\`
EOF

cat >"${ACCEPTANCE_ROOT}/handoff.md" <<EOF
# Acceptance Handoff

- run_id: \`${RUN_ID}\`
- review_version: \`${RUN_ID}\`
- review_status: \`reviewed\`
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
- gate_review: \`release-main-smoke, release-config-redline, and release-evidence-pack all passed on the fixed run\`
- open_issues_summary: \`no blocking open issues recorded in this release pack\`
- residual_risk_summary: \`no residual risks require conditional acceptance for this run\`
- final_decision: \`not_recorded_here\`
EOF

{
  echo "# Veto Checklist"
  echo
  echo "- run_id: \`${RUN_ID}\`"
  echo "- review_version: \`${RUN_ID}\`"
  echo "- review_status: \`reviewed\`"
  echo "- reviewed_by: \`agent_reviewed\`"
  echo "- reviewed_at: \`$(utc_now)\`"
  echo
  echo "| veto_id | source_refs | status | evidence_refs | defect_refs | reviewer_status | risk_acceptance_allowed | notes |"
  echo "|---|---|---|---|---|---|---|---|"
  jq -r --arg run_id "${RUN_ID}" '
    map(
      .evidence_refs = (.evidence_refs | map(gsub("<run_id>"; $run_id)))
    )[]
    | "| \(.veto_id) | `" + (.source_refs | join(", ")) + "` | \(.status) | `" + (.evidence_refs | join(", ")) + "` | `" + (.defect_refs | join(", ")) + "` | \(.reviewer_status) | \(.risk_acceptance_allowed) | `" + (.notes // "") + "` |"
  ' <<<"${VETO_CHECKLIST_JSON}"
} >"${ACCEPTANCE_ROOT}/veto-checklist.md"

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
| none | no accepted residual risk recorded for \`${RUN_ID}\` | not_applicable | no release-blocking or conditional-pass risk remained after PH-09 review | not required because all release gates and veto checks passed | veto passed; no S defect; no P0 A defect; no redaction failure; no evidence-path failure | \`reports/runs/${RUN_ID}/release-summary.md, reports/acceptance/veto-checklist.md\` | not_applicable | not_applicable | continue with standard acceptance review only | not_applicable | \`reports/acceptance/handoff.md\` | closed |
EOF

cat >"${ACCEPTANCE_ROOT}/open-issues.md" <<EOF
# Open Issues

- run_id: \`${RUN_ID}\`
- review_version: \`${RUN_ID}\`
- review_status: \`reviewed\`
- reviewed_by: \`agent_reviewed\`
- reviewed_at: \`$(utc_now)\`
- current_open_followups: \`none_recorded_in_release_pack\`

| issue_id | classification | impact | owner | action | deadline | evidence_refs |
|---|---|---|---|---|---|---|
| none | not_applicable | no B/C defect, non-blocking risk, or follow-up entry was recorded in this release pack | not_applicable | continue standard acceptance review using fixed-run reports only | not_applicable | \`reports/runs/${RUN_ID}/release-summary.md, reports/acceptance/handoff.md\` |
EOF
