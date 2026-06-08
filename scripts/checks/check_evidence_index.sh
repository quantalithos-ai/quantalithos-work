#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_evidence_index.sh --run-id <run_id> --artifact-root <path> --report-root <path>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

report_path="${REPORT_ROOT}/evidence-index.md"
json_path="${REPORT_ROOT}/evidence-index.json"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-$(default_artifact_root "${RUN_ID}")}"

[[ -f "${report_path}" ]] || die "evidence index markdown missing: ${report_path}"
[[ -f "${json_path}" ]] || die "evidence index json missing: ${json_path}"

entry_count="$(jq 'length' "${json_path}")"
cfg_ok="$(jq '[ .[] | select(.evidence_id == "EV-WORK-CFG-017") ] | length > 0' "${json_path}")"
nfr_ok="$(jq '[ .[] | select(.evidence_id == "EV-WORK-NFR-005") ] | length > 0' "${json_path}")"
ops_ok="$(jq '[ .[] | select(.evidence_id == "EV-WORK-OPS-006") ] | length > 0' "${json_path}")"
required_fields_ok="$(
  jq '
    all(
      .[];
      (.evidence_id? // "") != ""
      and ((.test_case_ids? // []) | length > 0)
      and ((.acceptance_ids? // []) | length > 0)
      and (.suite? // "") != ""
      and (.run_id? // "") != ""
      and ((.artifact_refs? // []) | length > 0)
      and ((.report_refs? // []) | length > 0)
      and ((.design_contract_refs? // []) | length > 0)
      and ((.redaction_status? // "") != "")
      and ((.review_status? // "") == "reviewed")
    )
  ' "${json_path}"
)"

status="passed"
if [[ "${cfg_ok}" != "true" || "${nfr_ok}" != "true" || "${ops_ok}" != "true" || "${required_fields_ok}" != "true" ]]; then
  status="failed"
fi

checks_dir="${ARTIFACT_ROOT}/checks"
ensure_dir "${checks_dir}"

jq -n \
  --arg run_id "${RUN_ID}" \
  --arg status "${status}" \
  --arg checked_at "$(utc_now)" \
  --argjson entry_count "${entry_count}" \
  --argjson cfg_017_present "${cfg_ok}" \
  --argjson nfr_005_present "${nfr_ok}" \
  --argjson ops_006_present "${ops_ok}" \
  --argjson required_fields_ok "${required_fields_ok}" \
  '{
    run_id: $run_id,
    status: $status,
    checked_at: $checked_at,
    entry_count: $entry_count,
    required_entries: {
      cfg_017_present: $cfg_017_present,
      nfr_005_present: $nfr_005_present,
      ops_006_present: $ops_006_present,
      required_fields_ok: $required_fields_ok
    }
  }' >"${checks_dir}/evidence-index.json"

[[ "${status}" == "passed" ]] || die "required evidence index entries missing"
