#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_config_source_summary.sh --run-id <run_id> --report-root <path>
       --config-profile <profile>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"
[[ -n "${CONFIG_PROFILE}" ]] || usage_arg_missing "--config-profile"

config_path="$(require_config_profile "${CONFIG_PROFILE}")"
config_digest="$(sha256_file "${config_path}")"
report_path="${REPORT_ROOT}/config-source-summary.md"

ensure_dir "${REPORT_ROOT}"

cat >"${report_path}" <<EOF
# Config Source Summary

- run_id: \`${RUN_ID}\`
- profile: \`${CONFIG_PROFILE}\`
- source_kind: \`json_profile\`
- source_path: \`$(artifact_rel "${config_path}")\`
- config_digest: \`${config_digest}\`
- value_redaction: \`ref_only\`
EOF
