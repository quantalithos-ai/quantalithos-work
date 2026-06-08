#!/usr/bin/env bash
set -euo pipefail

readonly WORK_PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

die() {
  echo "error: $*" >&2
  exit 1
}

usage_arg_missing() {
  die "missing required argument: $1"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

parse_common_args() {
  RUN_ID=""
  ARTIFACT_ROOT=""
  REPORT_ROOT=""
  CONFIG_PROFILE=""
  SUITE=""
  REPLAY_BUNDLE_REF=""
  ACCEPTANCE_ROOT=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --run-id)
        RUN_ID="${2:-}"
        shift 2
        ;;
      --artifact-root)
        ARTIFACT_ROOT="${2:-}"
        shift 2
        ;;
      --report-root)
        REPORT_ROOT="${2:-}"
        shift 2
        ;;
      --config-profile)
        CONFIG_PROFILE="${2:-}"
        shift 2
        ;;
      --suite)
        SUITE="${2:-}"
        shift 2
        ;;
      --replay-bundle-ref)
        REPLAY_BUNDLE_REF="${2:-}"
        shift 2
        ;;
      --acceptance-root)
        ACCEPTANCE_ROOT="${2:-}"
        shift 2
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
  done
}

default_artifact_root() {
  local run_id="$1"
  echo "${WORK_PROJECT_ROOT}/artifacts/test/${run_id}"
}

default_report_root() {
  local run_id="$1"
  echo "${WORK_PROJECT_ROOT}/reports/runs/${run_id}"
}

default_acceptance_root() {
  echo "${WORK_PROJECT_ROOT}/reports/acceptance"
}

ensure_dir() {
  mkdir -p "$1"
}

normalize_text_file_eof() {
  local path="$1"
  local tmp
  tmp="$(mktemp)"

  awk '
    { lines[NR] = $0 }
    END {
      last = NR
      while (last > 0 && lines[last] == "") {
        last--
      }
      for (i = 1; i <= last; i++) {
        print lines[i]
      }
    }
  ' "$path" >"$tmp"

  mv "$tmp" "$path"
}

json_escape() {
  jq -Rn --arg value "$1" '$value'
}

utc_now() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

sha256_file() {
  sha256sum "$1" | awk '{print $1}'
}

assert_no_latest_path() {
  local value="$1"
  if [[ "$value" == *"/latest"* || "$value" == *"latest/"* || "$value" == "latest" ]]; then
    die "forbidden latest path detected: $value"
  fi
}

assert_run_scoped_path() {
  local path_value="$1"
  local run_id="$2"
  if [[ "$path_value" != *"${run_id}"* ]]; then
    die "path does not include run_id ${run_id}: ${path_value}"
  fi
}

artifact_rel() {
  local abs_path="$1"
  realpath --relative-to="$WORK_PROJECT_ROOT" "$abs_path"
}

write_json() {
  local output_path="$1"
  local json_text="$2"
  printf '%s\n' "$json_text" >"$output_path"
}

suite_artifact_dir() {
  local artifact_root="$1"
  local suite="$2"
  echo "${artifact_root}/suites/${suite}"
}

suite_report_path() {
  local report_root="$1"
  local suite="$2"
  echo "${report_root}/suites/${suite}.md"
}

run_cargo_capture() {
  local suite_dir="$1"
  local command_label="$2"
  shift 2

  local stdout_path="${suite_dir}/stdout.log"
  local stderr_path="${suite_dir}/stderr.log"
  local started_at ended_at exit_code status
  started_at="$(utc_now)"

  set +e
  "$@" >"${stdout_path}" 2>"${stderr_path}"
  exit_code=$?
  set -e

  normalize_text_file_eof "${stdout_path}"
  normalize_text_file_eof "${stderr_path}"

  ended_at="$(utc_now)"
  if [[ ${exit_code} -eq 0 ]]; then
    status="passed"
  else
    status="failed"
  fi

  jq -n \
    --arg command_label "${command_label}" \
    --arg status "${status}" \
    --arg started_at "${started_at}" \
    --arg ended_at "${ended_at}" \
    --argjson exit_code "${exit_code}" \
    '{
      command_label: $command_label,
      status: $status,
      started_at: $started_at,
      ended_at: $ended_at,
      exit_code: $exit_code
    }'
}

write_failure_reason() {
  local path="$1"
  local suite="$2"
  local reason="$3"
  jq -n \
    --arg suite "${suite}" \
    --arg reason "${reason}" \
    --arg recorded_at "$(utc_now)" \
    '{
      suite: $suite,
      reason: $reason,
      recorded_at: $recorded_at
    }' >"$path"
  normalize_text_file_eof "$path"
}

write_context_json() {
  local artifact_root="$1"
  local run_id="$2"
  local suite="$3"
  local config_profile="$4"
  local config_path="$5"
  ensure_dir "${artifact_root}/meta"

  local config_digest
  config_digest="$(sha256_file "${config_path}")"

  jq -n \
    --arg run_id "${run_id}" \
    --arg suite "${suite}" \
    --arg config_profile "${config_profile}" \
    --arg config_path "$(artifact_rel "${config_path}")" \
    --arg config_digest "${config_digest}" \
    --arg generated_at "$(utc_now)" \
    '{
      run_id: $run_id,
      suite: $suite,
      config_profile: $config_profile,
      config_path: $config_path,
      config_digest: $config_digest,
      generated_at: $generated_at
    }' >"${artifact_root}/meta/context.json"
  normalize_text_file_eof "${artifact_root}/meta/context.json"
}

config_profile_path() {
  local profile="$1"
  echo "${WORK_PROJECT_ROOT}/config/profiles/${profile}.json"
}

require_config_profile() {
  local profile="$1"
  local path
  path="$(config_profile_path "${profile}")"
  [[ -f "${path}" ]] || die "config profile not found: ${path}"
  echo "${path}"
}

append_suite_report_line() {
  local output="$1"
  shift
  printf '%s\n' "$*" >>"${output}"
}

write_suite_markdown() {
  local output="$1"
  local title="$2"
  local status="$3"
  local run_id="$4"
  local suite="$5"
  local config_profile="$6"
  local report_json_rel="$7"
  local stdout_rel="$8"
  local stderr_rel="$9"
  local failure_rel="${10}"

  cat >"${output}" <<EOF
# ${title}

- status: \`${status}\`
- run_id: \`${run_id}\`
- suite: \`${suite}\`
- config_profile: \`${config_profile}\`
- artifact_report: \`${report_json_rel}\`
- stdout: \`${stdout_rel}\`
- stderr: \`${stderr_rel}\`
- failure_reason: \`${failure_rel}\`
EOF
}

init_suite_layout() {
  local artifact_root="$1"
  local report_root="$2"
  local suite="$3"

  ensure_dir "${artifact_root}"
  ensure_dir "${artifact_root}/checks"
  ensure_dir "${artifact_root}/redaction-scan"
  ensure_dir "${artifact_root}/meta"
  ensure_dir "$(suite_artifact_dir "${artifact_root}" "${suite}")"
  ensure_dir "${report_root}"
  ensure_dir "${report_root}/suites"
  ensure_dir "${report_root}/evidence"
}

finalize_suite_report() {
  local artifact_root="$1"
  local report_root="$2"
  local run_id="$3"
  local suite="$4"
  local config_profile="$5"
  local title="$6"

  local suite_dir report_json failure_json stdout_log stderr_log status
  suite_dir="$(suite_artifact_dir "${artifact_root}" "${suite}")"
  report_json="${suite_dir}/report.json"
  failure_json="${suite_dir}/failure-reason.json"
  stdout_log="${suite_dir}/stdout.log"
  stderr_log="${suite_dir}/stderr.log"
  status="$(jq -r '.status' "${report_json}")"

  write_suite_markdown \
    "$(suite_report_path "${report_root}" "${suite}")" \
    "${title}" \
    "${status}" \
    "${run_id}" \
    "${suite}" \
    "${config_profile}" \
    "$(artifact_rel "${report_json}")" \
    "$(artifact_rel "${stdout_log}")" \
    "$(artifact_rel "${stderr_log}")" \
    "$(artifact_rel "${failure_json}")"
}

write_suite_result_json() {
  local output_path="$1"
  local suite="$2"
  local run_id="$3"
  local config_profile="$4"
  local status="$5"
  local command_label="$6"
  local exit_code="$7"

  jq -n \
    --arg suite "${suite}" \
    --arg run_id "${run_id}" \
    --arg config_profile "${config_profile}" \
    --arg status "${status}" \
    --arg command_label "${command_label}" \
    --arg started_at "$(utc_now)" \
    --arg finished_at "$(utc_now)" \
    --argjson exit_code "${exit_code}" \
    '{
      suite: $suite,
      run_id: $run_id,
      config_profile: $config_profile,
      status: $status,
      command_label: $command_label,
      started_at: $started_at,
      finished_at: $finished_at,
      exit_code: $exit_code
    }' >"${output_path}"
}

record_suite_command() {
  local output_path="$1"
  local suite="$2"
  local package_name="$3"
  local target_kind="$4"
  local target_name="$5"
  local test_name="$6"
  local command="$7"

  jq -n \
    --arg suite "${suite}" \
    --arg package_name "${package_name}" \
    --arg target_kind "${target_kind}" \
    --arg target_name "${target_name}" \
    --arg test_name "${test_name}" \
    --arg command "${command}" \
    '{
      suite: $suite,
      package_name: $package_name,
      target_kind: $target_kind,
      target_name: $target_name,
      test_name: $test_name,
      command: $command
    }' >"${output_path}"
  normalize_text_file_eof "${output_path}"
}
