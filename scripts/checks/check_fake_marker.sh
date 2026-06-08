#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/.." && pwd)/lib/release_common.sh"

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/checks/check_fake_marker.sh --run-id <run_id> --artifact-root <path> --report-root <path>
EOF
  exit 0
fi

require_command jq

parse_common_args "$@"

[[ -n "${RUN_ID}" ]] || usage_arg_missing "--run-id"
[[ -n "${ARTIFACT_ROOT}" ]] || usage_arg_missing "--artifact-root"
[[ -n "${REPORT_ROOT}" ]] || usage_arg_missing "--report-root"

ensure_dir "${ARTIFACT_ROOT}/checks"

profiles=(local-dev ci-test integration-like operations-replay)
results_json='[]'

for profile in "${profiles[@]}"; do
  config_path="$(require_config_profile "${profile}")"
  profile_result="$(
    jq -c \
      --arg profile "${profile}" \
      '
      def has_configured_ref:
        (.endpoint_ref? // .credential_ref? // .target_ref?) != null;
      {
        profile: $profile,
        fake_marker_required:
          (
            [
              .external.identity.adapter_kind,
              .external.method_library.adapter_kind,
              .external.source_work.adapter_kind,
              .external.evidence.adapter_kind,
              .external.process_timebox.adapter_kind,
              .outbox.publisher.adapter_kind,
              .handoff.trace_target.adapter_kind,
              .handoff.archive_target.adapter_kind
            ] | any(. == "fake")
          ),
        configured_refs_present:
          (
            [
              .external.identity,
              .external.method_library,
              .external.source_work,
              .external.evidence,
              .external.process_timebox,
              .outbox.publisher,
              .handoff.trace_target,
              .handoff.archive_target
            ] | all(if .adapter_kind == "configured" then has_configured_ref else true end)
          )
      }
      ' "${config_path}"
  )"
  results_json="$(jq -c --argjson item "${profile_result}" '. + [$item]' <<<"${results_json}")"
done

status="$(jq -r 'if all(.[]; .configured_refs_present == true) then "passed" else "failed" end' <<<"${results_json}")"

jq -n \
  --arg run_id "${RUN_ID}" \
  --arg status "${status}" \
  --arg checked_at "$(utc_now)" \
  --argjson profiles "${results_json}" \
  '{
    run_id: $run_id,
    status: $status,
    checked_at: $checked_at,
    profiles: $profiles
  }' >"${ARTIFACT_ROOT}/checks/fake-marker.json"

[[ "${status}" == "passed" ]] || die "configured adapter missing ref or fake marker invariant failed"
