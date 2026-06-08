#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"
source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_data.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/reports/build_evidence_index.sh --run-id <run_id> --report-root reports/runs/<run_id>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

ARTIFACT_ROOT="${ARTIFACT_ROOT:-$(default_artifact_root "${RUN_ID}")}"
ensure_dir "${REPORT_ROOT}"
ensure_dir "${REPORT_ROOT}/evidence"
ensure_dir "${ARTIFACT_ROOT}"

evidence_json="$(
  jq \
    --arg run_id "${RUN_ID}" \
    --arg artifact_root "$(artifact_rel "${ARTIFACT_ROOT}")" \
    --arg report_root "$(artifact_rel "${REPORT_ROOT}")" \
    '
    map(
      . + {
        run_id: $run_id,
        artifact_refs: ["\($artifact_root)/suites/\(.suite)/report.json"],
        report_refs: ["\($report_root)/\(.suite).md", "\($report_root)/gate-results.md"]
      }
    )
    ' <<<"${P0_EVIDENCE_JSON}"
)"

printf '%s\n' "${evidence_json}" >"${REPORT_ROOT}/evidence-index.json"
printf '%s\n' "${evidence_json}" >"${ARTIFACT_ROOT}/evidence-index.json"
normalize_text_file_eof "${REPORT_ROOT}/evidence-index.json"
normalize_text_file_eof "${ARTIFACT_ROOT}/evidence-index.json"

{
  echo "# Evidence Index"
  echo
  echo "- run_id: \`${RUN_ID}\`"
  echo
  jq -r '
    .[] |
    "## \(.evidence_id)\n" +
    "- test_case_ids: `" + (.test_case_ids | join(", ")) + "`\n" +
    "- acceptance_ids: `" + (.acceptance_ids | join(", ")) + "`\n" +
    "- suite: `\(.suite)`\n" +
    "- run_id: `\(.run_id)`\n" +
    "- artifact_refs: `" + (.artifact_refs | join(", ")) + "`\n" +
    "- report_refs: `" + (.report_refs | join(", ")) + "`\n" +
    "- design_contract_refs: `" + (.design_contract_refs | join(", ")) + "`\n" +
    "- redaction_status: `\(.redaction_status)`\n" +
    "- review_status: `\(.review_status)`\n"
  ' <<<"${evidence_json}"
} >"${REPORT_ROOT}/evidence-index.md"
normalize_text_file_eof "${REPORT_ROOT}/evidence-index.md"

while IFS= read -r evidence_id; do
  jq -r --arg evidence_id "${evidence_id}" '
    map(select(.evidence_id == $evidence_id))[]
    | "# \(.evidence_id)\n\n"
    + "- test_case_ids: `" + (.test_case_ids | join(", ")) + "`\n"
    + "- acceptance_ids: `" + (.acceptance_ids | join(", ")) + "`\n"
    + "- suite: `\(.suite)`\n"
    + "- run_id: `\(.run_id)`\n"
    + "- artifact_refs: `" + (.artifact_refs | join(", ")) + "`\n"
    + "- report_refs: `" + (.report_refs | join(", ")) + "`\n"
    + "- design_contract_refs: `" + (.design_contract_refs | join(", ")) + "`\n"
    + "- redaction_status: `\(.redaction_status)`\n"
    + "- review_status: `\(.review_status)`\n"
  ' <<<"${evidence_json}" >"${REPORT_ROOT}/evidence/${evidence_id}.md"
  normalize_text_file_eof "${REPORT_ROOT}/evidence/${evidence_id}.md"
done < <(jq -r '.[].evidence_id' <<<"${evidence_json}")
