#!/usr/bin/env bash
set -u

MODE="all"
STRICT_NETWORK=0
SEND_TEST_EVENT=0
ORG="${SENTRY_ORG:-}"
PROJECT="${SENTRY_PROJECT:-}"
RELEASE_OVERRIDE=""
JSON_OUTPUT=0
DEFAULT_CWD="$(pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$DEFAULT_CWD"
REPO_ROOT_FROM_ARG=0
SEARCH_TOOL=""
PYTHON_BIN=""
STATE_FILE=""
REQUIRE_HOST_NETWORK=0
AUTO_SLUG_DISCOVERY=1

TMP_RESULTS="$(mktemp)"
TMP_RUNTIME="$(mktemp)"
trap 'rm -f "$TMP_RESULTS" "$TMP_RUNTIME"' EXIT

CMD_OUTPUT=""
CMD_RC=0
QUERY_COUNT=""

# Runtime metrics used by verify-loop state snapshots.
METRIC_UNRESOLVED_COUNT=""
METRIC_HIGH_SEVERITY_COUNT=""
METRIC_RELEASE_REGRESSION_COUNT=""
METRIC_NEW_LAST_DAY_COUNT=""

AUTO_RECOVERY_NOTES=""

usage() {
  cat <<'EOF_USAGE'
Usage:
  sentry_monitor.sh [options]

Options:
  --mode setup-preflight|local-smoke|prod-health|release-gate|issue-triage|verify-loop|all
  --strict-network                  Treat network/API errors as FAIL (default: false)
  --send-test-event                 Send synthetic smoke event in local-smoke mode
  --org <slug>                      Sentry organization slug (or use SENTRY_ORG)
  --project <slug>                  Sentry project slug (or use SENTRY_PROJECT)
  --release <value>                 Override expected release (default derives from repo)
  --repo-root <path>                Repo root path (default: auto-detected git root)
  --state-file <path>               State file path for verify-loop checkpoints
  --require-host-network            Fail API checks if network/DNS is unavailable
  --auto-slug-discovery             Enable org/project auto-discovery (default: true)
  --no-auto-slug-discovery          Disable org/project auto-discovery
  --json                            Emit JSON summary output
  -h, --help                        Show help
EOF_USAGE
}

sanitize_field() {
  printf '%s' "$1" | tr '\t\n\r' '   '
}

record_result() {
  # mode, status, evidence, risks, next_commands
  local mode="$1"
  local status="$2"
  local evidence
  local risks
  local next_commands
  evidence="$(sanitize_field "${3:-n/a}")"
  risks="$(sanitize_field "${4:-n/a}")"
  next_commands="$(sanitize_field "${5:-none}")"
  printf '%s\t%s\t%s\t%s\t%s\n' "$mode" "$status" "$evidence" "$risks" "$next_commands" >>"$TMP_RESULTS"
}

append_note() {
  if [ -z "$1" ]; then
    printf '%s' "$2"
  else
    printf '%s; %s' "$1" "$2"
  fi
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

init_search_tool() {
  if command_exists rg; then
    SEARCH_TOOL="rg"
    return 0
  fi
  if command_exists grep; then
    SEARCH_TOOL="grep"
    return 0
  fi
  return 1
}

init_python_tool() {
  if command_exists python3; then
    PYTHON_BIN="python3"
    return 0
  fi
  if command_exists python; then
    PYTHON_BIN="python"
    return 0
  fi
  return 1
}

resolve_default_repo_root() {
  local cwd_root script_root
  cwd_root="$(git -C "$DEFAULT_CWD" rev-parse --show-toplevel 2>/dev/null || true)"
  if [ -n "$cwd_root" ]; then
    REPO_ROOT="$cwd_root"
    return 0
  fi

  script_root="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
  if [ -n "$script_root" ]; then
    REPO_ROOT="$script_root"
    return 0
  fi

  REPO_ROOT="$DEFAULT_CWD"
  return 0
}

load_local_env_defaults() {
  local env_file="$REPO_ROOT/.env"
  if [ ! -f "$env_file" ]; then
    return 0
  fi

  while IFS= read -r raw_line || [ -n "$raw_line" ]; do
    local line="$raw_line"
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"

    [ -z "$line" ] && continue
    case "$line" in
      \#*) continue ;;
    esac

    line="${line#export }"
    case "$line" in
      *=*) ;;
      *) continue ;;
    esac

    local key="${line%%=*}"
    local value="${line#*=}"
    key="${key#"${key%%[![:space:]]*}"}"
    key="${key%"${key##*[![:space:]]}"}"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"

    case "$key" in
      [A-Za-z_][A-Za-z0-9_]*) ;;
      *) continue ;;
    esac

    if [ -n "${!key:-}" ]; then
      continue
    fi

    case "$value" in
      \"*\") value="${value#\"}"; value="${value%\"}" ;;
      \'*\') value="${value#\'}"; value="${value%\'}" ;;
    esac

    export "$key=$value"
  done < "$env_file"
}

is_truthy() {
  local value="${1:-}"
  value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  case "$value" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

first_non_empty_line() {
  awk 'NF { print; exit }'
}

quiet_regex_match_stdin() {
  local pattern="$1"
  if [ "$SEARCH_TOOL" = "rg" ]; then
    rg -qi -- "$pattern"
  else
    grep -Eqi -- "$pattern"
  fi
}

quiet_regex_match_file() {
  local pattern="$1"
  local file="$2"
  if [ "$SEARCH_TOOL" = "rg" ]; then
    rg -q -- "$pattern" "$file"
  else
    grep -Eq -- "$pattern" "$file"
  fi
}

quiet_fixed_match_file() {
  local pattern="$1"
  local file="$2"
  if [ "$SEARCH_TOOL" = "rg" ]; then
    rg -q --fixed-strings -- "$pattern" "$file"
  else
    grep -Fq -- "$pattern" "$file"
  fi
}

first_regex_match_line() {
  local pattern="$1"
  local file="$2"
  if [ "$SEARCH_TOOL" = "rg" ]; then
    rg -n -- "$pattern" "$file" | head -n 1 | cut -d: -f2-
  else
    grep -En -- "$pattern" "$file" | head -n 1 | cut -d: -f2-
  fi
}

is_network_error() {
  local text="$1"
  if printf '%s' "$text" | quiet_regex_match_stdin \
    "Could not resolve|Temporary failure|timed out|timeout|connection refused|connection reset|network is unreachable|TLS|SSL|Could not resolve host"; then
    return 0
  fi
  return 1
}

is_permission_error() {
  local text="$1"
  if printf '%s' "$text" | quiet_regex_match_stdin "You do not have permission|http status: 403|forbidden|unauthorized|401"; then
    return 0
  fi
  return 1
}

is_org_not_found_error() {
  local text="$1"
  if printf '%s' "$text" | quiet_regex_match_stdin "organization not found|org not found|unknown organization"; then
    return 0
  fi
  return 1
}

is_project_not_found_error() {
  local text="$1"
  if printf '%s' "$text" | quiet_regex_match_stdin "project not found|unknown project"; then
    return 0
  fi
  return 1
}

run_cmd_capture() {
  CMD_OUTPUT="$("$@" 2>&1)"
  CMD_RC=$?
}

extract_first_version() {
  local file="$1"
  local pattern="$2"
  if [ ! -f "$file" ]; then
    printf ''
    return
  fi

  local line
  line="$(first_regex_match_line "$pattern" "$file")"
  if [ -z "$line" ]; then
    printf ''
    return
  fi

  printf '%s' "$line" | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/'
}

extract_cargo_version() {
  local file="$1"
  if [ ! -f "$file" ]; then
    printf ''
    return
  fi
  local line
  line="$(first_regex_match_line '^version[[:space:]]*=[[:space:]]*"[^"]+"' "$file")"
  if [ -z "$line" ]; then
    printf ''
    return
  fi
  printf '%s' "$line" | sed -E 's/version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/'
}

extract_package_name() {
  local file="$1"
  if [ ! -f "$file" ]; then
    printf ''
    return
  fi

  local line
  line="$(first_regex_match_line '"name"[[:space:]]*:[[:space:]]*"[^"]+"' "$file")"
  if [ -z "$line" ]; then
    printf ''
    return
  fi
  printf '%s' "$line" | sed -E 's/.*"name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/'
}

has_env_key() {
  local key="$1"
  if [ -n "${!key:-}" ]; then
    return 0
  fi
  if [ -f "$REPO_ROOT/.env" ] && quiet_regex_match_file "^${key}=" "$REPO_ROOT/.env"; then
    return 0
  fi
  return 1
}

check_fixed_string() {
  local file="$1"
  local pattern="$2"
  if [ ! -f "$file" ]; then
    return 1
  fi
  quiet_fixed_match_file "$pattern" "$file"
}

count_issue_rows() {
  printf '%s\n' "$1" | awk -F'|' '
    /^\|/ {
      for (i=1; i<=NF; i++) {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $i)
      }
      if ($2 == "" || $2 == "ID") next
      c++
    }
    END { print c+0 }
  '
}

extract_table_column_values() {
  local text="$1"
  local column="$2"
  printf '%s\n' "$text" | awk -F'|' -v col="$column" '
    /^\|/ {
      for (i=1; i<=NF; i++) {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $i)
      }
      if ($2 == "" || $2 == "ID") next
      if ($col != "") print $col
    }
  ' | sed '/^[[:space:]]*$/d' | sort -u
}

csv_from_lines() {
  if [ -z "$1" ]; then
    printf ''
    return
  fi
  printf '%s\n' "$1" | paste -sd ', ' -
}

build_command_for_mode() {
  local mode_name="$1"
  local cmd
  cmd="bash $SCRIPT_DIR/sentry_monitor.sh --mode $mode_name --repo-root $REPO_ROOT"
  if [ -n "$ORG" ]; then
    cmd="$cmd --org $ORG"
  fi
  if [ -n "$PROJECT" ]; then
    cmd="$cmd --project $PROJECT"
  fi
  if [ -n "$RELEASE_OVERRIDE" ]; then
    cmd="$cmd --release $RELEASE_OVERRIDE"
  fi
  if [ "$REQUIRE_HOST_NETWORK" -eq 0 ]; then
    cmd="$cmd --require-host-network"
  fi
  printf '%s' "$cmd"
}

api_network_failure_result() {
  local mode_name="$1"
  local action="$2"
  local rerun_cmd
  rerun_cmd="$(build_command_for_mode "$mode_name")"

  if [ "$REQUIRE_HOST_NETWORK" -eq 1 ] || [ "$STRICT_NETWORK" -eq 1 ]; then
    record_result "$mode_name" "FAIL" \
      "${action} failed due to network/DNS unavailability in the current execution context." \
      "Host-network access is required for Sentry API verification in this run." \
      "$rerun_cmd"
  else
    record_result "$mode_name" "PARTIAL" \
      "${action} is inconclusive because network/DNS is unavailable in the current execution context." \
      "Current context may be sandboxed; this does not prove Sentry is misconfigured." \
      "$rerun_cmd"
  fi
}

api_auth_failure_result() {
  local mode_name="$1"
  local action="$2"
  record_result "$mode_name" "FAIL" \
    "${action} failed with permission/auth error." \
    "Token likely lacks required scopes for this API mode." \
    "sentry-cli info --no-defaults"
}

ensure_sentry_api_connectivity() {
  local mode_name="$1"
  run_cmd_capture sentry-cli info --no-defaults
  if [ "$CMD_RC" -ne 0 ]; then
    if is_network_error "$CMD_OUTPUT"; then
      api_network_failure_result "$mode_name" "Sentry API reachability precheck"
      return 1
    fi
    if is_permission_error "$CMD_OUTPUT"; then
      api_auth_failure_result "$mode_name" "Sentry API reachability precheck"
      return 1
    fi
    local reason
    reason="$(printf '%s' "$CMD_OUTPUT" | first_non_empty_line)"
    [ -z "$reason" ] && reason="unknown sentry-cli error"
    record_result "$mode_name" "FAIL" \
      "Sentry API reachability precheck failed." \
      "$reason" \
      "sentry-cli info --no-defaults --log-level=debug"
    return 1
  fi
  return 0
}

discover_org_candidates() {
  run_cmd_capture sentry-cli organizations list
  if [ "$CMD_RC" -ne 0 ]; then
    return 1
  fi
  extract_table_column_values "$CMD_OUTPUT" 4
}

discover_project_candidates() {
  local org_slug="$1"
  run_cmd_capture sentry-cli projects list --org "$org_slug"
  if [ "$CMD_RC" -ne 0 ]; then
    return 1
  fi
  extract_table_column_values "$CMD_OUTPUT" 3
}

append_auto_recovery_note() {
  local text="$1"
  AUTO_RECOVERY_NOTES="$(append_note "$AUTO_RECOVERY_NOTES" "$text")"
}

resolve_org_slug() {
  local mode_name="$1"
  local org_candidates candidates_csv count

  org_candidates="$(discover_org_candidates)"
  if [ "$CMD_RC" -ne 0 ]; then
    if is_network_error "$CMD_OUTPUT"; then
      api_network_failure_result "$mode_name" "Organization discovery"
      return 1
    fi
    if is_permission_error "$CMD_OUTPUT"; then
      api_auth_failure_result "$mode_name" "Organization discovery"
      return 1
    fi
    record_result "$mode_name" "FAIL" \
      "Organization discovery failed." \
      "Unable to read organization list from sentry-cli." \
      "sentry-cli organizations list --log-level=debug"
    return 1
  fi

  count="$(printf '%s\n' "$org_candidates" | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')"
  candidates_csv="$(csv_from_lines "$org_candidates")"

  if [ -z "$ORG" ]; then
    if [ "$AUTO_SLUG_DISCOVERY" -eq 1 ] && [ "$count" -eq 1 ]; then
      ORG="$org_candidates"
      append_auto_recovery_note "Auto-selected org='$ORG' (single candidate)."
      return 0
    fi
    if [ "$count" -gt 1 ]; then
      record_result "$mode_name" "FAIL" \
        "Organization slug is missing and multiple candidates were discovered." \
        "Ambiguous org selection: $candidates_csv" \
        "$(build_command_for_mode "$mode_name") --org <org-slug>"
      return 1
    fi
    record_result "$mode_name" "FAIL" \
      "Organization slug is missing and no usable candidate was discovered." \
      "Cannot proceed with API checks without --org or SENTRY_ORG." \
      "$(build_command_for_mode "$mode_name") --org <org-slug>"
    return 1
  fi

  if printf '%s\n' "$org_candidates" | grep -Fxq "$ORG"; then
    return 0
  fi

  record_result "$mode_name" "FAIL" \
    "Organization slug '$ORG' was not found." \
    "Candidate org slugs: ${candidates_csv:-none}" \
    "sentry-cli organizations list"
  return 1
}

resolve_project_slug() {
  local mode_name="$1"
  local project_candidates candidates_csv count

  project_candidates="$(discover_project_candidates "$ORG")"
  if [ "$CMD_RC" -ne 0 ]; then
    if is_network_error "$CMD_OUTPUT"; then
      api_network_failure_result "$mode_name" "Project discovery"
      return 1
    fi
    if is_permission_error "$CMD_OUTPUT"; then
      api_auth_failure_result "$mode_name" "Project discovery"
      return 1
    fi
    if is_org_not_found_error "$CMD_OUTPUT"; then
      record_result "$mode_name" "FAIL" \
        "Project discovery failed because organization '$ORG' was not found." \
        "Organization slug is invalid for this token/context." \
        "sentry-cli organizations list"
      return 1
    fi
    record_result "$mode_name" "FAIL" \
      "Project discovery failed for org '$ORG'." \
      "Unable to read project list from sentry-cli." \
      "sentry-cli projects list --org $ORG --log-level=debug"
    return 1
  fi

  count="$(printf '%s\n' "$project_candidates" | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')"
  candidates_csv="$(csv_from_lines "$project_candidates")"

  if [ -z "$PROJECT" ]; then
    if [ "$AUTO_SLUG_DISCOVERY" -eq 1 ] && [ "$count" -eq 1 ]; then
      PROJECT="$project_candidates"
      append_auto_recovery_note "Auto-selected project='$PROJECT' (single candidate in org '$ORG')."
      return 0
    fi
    if [ "$count" -gt 1 ]; then
      record_result "$mode_name" "FAIL" \
        "Project slug is missing and multiple candidates were discovered under org '$ORG'." \
        "Ambiguous project selection: $candidates_csv" \
        "$(build_command_for_mode "$mode_name") --project <project-slug>"
      return 1
    fi
    record_result "$mode_name" "FAIL" \
      "Project slug is missing and no usable candidate was discovered under org '$ORG'." \
      "Cannot proceed with API checks without --project or SENTRY_PROJECT." \
      "$(build_command_for_mode "$mode_name") --project <project-slug>"
    return 1
  fi

  if printf '%s\n' "$project_candidates" | grep -Fxq "$PROJECT"; then
    return 0
  fi

  record_result "$mode_name" "FAIL" \
    "Project slug '$PROJECT' was not found in org '$ORG'." \
    "Candidate project slugs: ${candidates_csv:-none}" \
    "sentry-cli projects list --org $ORG"
  return 1
}

prepare_api_context() {
  local mode_name="$1"

  if ! ensure_sentry_api_connectivity "$mode_name"; then
    return 1
  fi

  if ! resolve_org_slug "$mode_name"; then
    return 1
  fi

  if ! resolve_project_slug "$mode_name"; then
    return 1
  fi

  return 0
}

query_issue_count() {
  local mode_name="$1"
  local query="$2"
  local max_rows="$3"
  QUERY_COUNT=""

  run_cmd_capture sentry-cli issues list \
    -o "$ORG" \
    -p "$PROJECT" \
    --query "$query" \
    --max-rows "$max_rows"

  if [ "$CMD_RC" -ne 0 ]; then
    return 1
  fi

  QUERY_COUNT="$(count_issue_rows "$CMD_OUTPUT")"
  return 0
}

expected_release() {
  local tauri_ver cargo_ver pkg_ver package_name app_name version_value
  if [ -n "${SENTRY_RELEASE:-}" ]; then
    printf '%s' "$SENTRY_RELEASE"
    return
  fi

  tauri_ver="$(extract_first_version "$REPO_ROOT/src-tauri/tauri.conf.json" '"version"[[:space:]]*:[[:space:]]*"[^"]+"')"
  cargo_ver="$(extract_cargo_version "$REPO_ROOT/src-tauri/Cargo.toml")"
  pkg_ver="$(extract_first_version "$REPO_ROOT/package.json" '"version"[[:space:]]*:[[:space:]]*"[^"]+"')"
  package_name="$(extract_package_name "$REPO_ROOT/package.json")"

  version_value=""
  if [ -n "$tauri_ver" ]; then
    version_value="$tauri_ver"
  elif [ -n "$cargo_ver" ]; then
    version_value="$cargo_ver"
  elif [ -n "$pkg_ver" ]; then
    version_value="$pkg_ver"
  fi

  app_name="${package_name:-app}"

  if [ -n "$version_value" ]; then
    printf '%s@%s' "$app_name" "$version_value"
    return
  fi
  printf 'unknown'
}

infer_result_status() {
  local overall="PASS"
  while IFS=$'\t' read -r mode status _evidence _risks _next; do
    [ -z "$mode" ] && continue
    if [ "$status" = "FAIL" ]; then
      overall="FAIL"
      break
    fi
    if [ "$status" = "PARTIAL" ] && [ "$overall" != "FAIL" ]; then
      overall="PARTIAL"
    fi
  done <"$TMP_RESULTS"
  printf '%s' "$overall"
}

compute_default_state_file() {
  local org_slug project_slug
  org_slug="${ORG:-unknown-org}"
  project_slug="${PROJECT:-unknown-project}"

  org_slug="$(printf '%s' "$org_slug" | tr -cs '[:alnum:]_-' '_')"
  project_slug="$(printf '%s' "$project_slug" | tr -cs '[:alnum:]_-' '_')"

  printf '%s/.agents/skills/sentry-monitoring/state/%s_%s.json' "$REPO_ROOT" "$org_slug" "$project_slug"
}

resolve_state_file_if_needed() {
  if [ -n "$STATE_FILE" ]; then
    return 0
  fi
  STATE_FILE="$(compute_default_state_file)"
  return 0
}

load_previous_state() {
  local state_path="$1"
  "$PYTHON_BIN" - "$state_path" <<'PY'
import json
import os
import sys

path = sys.argv[1]
if not os.path.exists(path):
    print("")
    print("")
    print("")
    print("")
    print("")
    sys.exit(0)

with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

print(data.get("checked_at", ""))
print(data.get("release", ""))
print(data.get("unresolved_count", ""))
print(data.get("high_severity_count", ""))
print(data.get("release_regression_count", ""))
PY
}

persist_state_snapshot() {
  local state_path="$1"
  local checked_at="$2"
  local release="$3"
  local unresolved="$4"
  local high_severity="$5"
  local release_regression="$6"

  "$PYTHON_BIN" - "$state_path" "$checked_at" "$release" "$unresolved" "$high_severity" "$release_regression" <<'PY'
import json
import os
import sys

path, checked_at, release, unresolved, high_severity, release_regression = sys.argv[1:7]
os.makedirs(os.path.dirname(path), exist_ok=True)

payload = {
    "checked_at": checked_at,
    "release": release,
    "unresolved_count": int(unresolved) if unresolved.isdigit() else unresolved,
    "high_severity_count": int(high_severity) if high_severity.isdigit() else high_severity,
    "release_regression_count": int(release_regression) if release_regression.isdigit() else release_regression,
}

with open(path, "w", encoding="utf-8") as fh:
    json.dump(payload, fh, indent=2)
PY
}

mode_setup_preflight() {
  local mode_name="setup-preflight"
  local notes=""
  local failed=0

  local lib_file="$REPO_ROOT/src-tauri/src/lib.rs"
  local cap_file="$REPO_ROOT/src-tauri/capabilities/default.json"
  local vite_file="$REPO_ROOT/vite.config.ts"
  local workflow_file="$REPO_ROOT/.github/workflows/build.yml"
  local codictate_layout=0

  if ! command_exists sentry-cli; then
    record_result "$mode_name" "FAIL" \
      "sentry-cli is not available on PATH." \
      "Setup checks cannot continue without sentry-cli." \
      "Install sentry-cli, then rerun setup-preflight."
    return
  fi

  if [ -f "$lib_file" ] && [ -f "$cap_file" ] && [ -f "$vite_file" ] && [ -f "$workflow_file" ]; then
    codictate_layout=1
  fi

  if [ "$codictate_layout" -eq 1 ]; then
    if ! check_fixed_string "$cap_file" '"sentry:default"'; then
      failed=1
      notes="$(append_note "$notes" "Missing sentry capability permission in src-tauri/capabilities/default.json")"
    fi

    if ! check_fixed_string "$lib_file" "SENTRY_DSN_ENV_VAR"; then
      failed=1
      notes="$(append_note "$notes" "Missing DSN env variable contract in src-tauri/src/lib.rs")"
    fi
    if ! check_fixed_string "$lib_file" "HANDY_DISABLE_SENTRY_ENV_VAR"; then
      failed=1
      notes="$(append_note "$notes" "Missing kill-switch env variable contract in src-tauri/src/lib.rs")"
    fi
    if ! check_fixed_string "$lib_file" "send_default_pii: false"; then
      failed=1
      notes="$(append_note "$notes" "Missing send_default_pii=false setting in src-tauri/src/lib.rs")"
    fi
    if ! check_fixed_string "$lib_file" "before_send: Some("; then
      failed=1
      notes="$(append_note "$notes" "Missing before_send scrub hook in src-tauri/src/lib.rs")"
    fi
    if ! check_fixed_string "$lib_file" "tauri_plugin_sentry::init(client)"; then
      failed=1
      notes="$(append_note "$notes" "Missing tauri-plugin-sentry initialization path in src-tauri/src/lib.rs")"
    fi

    if ! check_fixed_string "$vite_file" "sentryVitePlugin"; then
      failed=1
      notes="$(append_note "$notes" "Missing @sentry/vite-plugin wiring in vite.config.ts")"
    fi
    if ! check_fixed_string "$vite_file" "SENTRY_AUTH_TOKEN"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_AUTH_TOKEN gate in vite.config.ts")"
    fi
    if ! check_fixed_string "$vite_file" "SENTRY_ORG"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_ORG gate in vite.config.ts")"
    fi
    if ! check_fixed_string "$vite_file" "SENTRY_PROJECT"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_PROJECT gate in vite.config.ts")"
    fi
    if ! check_fixed_string "$vite_file" "SENTRY_RELEASE"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_RELEASE handling in vite.config.ts")"
    fi

    if ! check_fixed_string "$workflow_file" "SENTRY_AUTH_TOKEN"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_AUTH_TOKEN wiring in .github/workflows/build.yml")"
    fi
    if ! check_fixed_string "$workflow_file" "SENTRY_ORG"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_ORG wiring in .github/workflows/build.yml")"
    fi
    if ! check_fixed_string "$workflow_file" "SENTRY_PROJECT"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_PROJECT wiring in .github/workflows/build.yml")"
    fi
    if ! check_fixed_string "$workflow_file" "SENTRY_DSN"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_DSN wiring in .github/workflows/build.yml for embedded DSN builds")"
    fi
    if ! check_fixed_string "$workflow_file" "SENTRY_RELEASE"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_RELEASE wiring in .github/workflows/build.yml")"
    fi
    if ! check_fixed_string "$workflow_file" "SENTRY_ENVIRONMENT"; then
      failed=1
      notes="$(append_note "$notes" "Missing SENTRY_ENVIRONMENT wiring in .github/workflows/build.yml")"
    fi
  else
    notes="$(append_note "$notes" "Repo layout is not Codictate/Tauri v2; skipped repo-specific wiring checks")"
  fi

  if ! has_env_key "SENTRY_DSN"; then
    notes="$(append_note "$notes" "SENTRY_DSN not found in current env or .env (may rely on embedded build-time DSN)")"
  fi
  if ! has_env_key "SENTRY_ENVIRONMENT"; then
    notes="$(append_note "$notes" "SENTRY_ENVIRONMENT not found in current env or .env (optional)")"
  fi
  if ! has_env_key "SENTRY_RELEASE"; then
    notes="$(append_note "$notes" "SENTRY_RELEASE not set locally (optional; runtime fallback exists)")"
  fi
  if ! has_env_key "HANDY_DISABLE_SENTRY"; then
    notes="$(append_note "$notes" "HANDY_DISABLE_SENTRY not set locally (optional kill-switch)")"
  fi

  if [ "$failed" -eq 1 ]; then
    record_result "$mode_name" "FAIL" \
      "Setup preflight found missing required Sentry wiring markers." \
      "$notes" \
      "Fix missing markers, then rerun: $(build_command_for_mode "$mode_name")"
    return
  fi

  if [ -n "$notes" ]; then
    record_result "$mode_name" "PARTIAL" \
      "Core wiring checks passed with non-blocking notes." \
      "$notes" \
      "Review notes and rerun: $(build_command_for_mode "$mode_name")"
  else
    record_result "$mode_name" "PASS" \
      "Core wiring and required env contracts are present." \
      "No immediate setup risk detected." \
      "Proceed with local-smoke and API modes."
  fi
}

mode_local_smoke() {
  local mode_name="local-smoke"
  local notes=""
  local dsn_value="${SENTRY_DSN:-}"
  local disable_value="${HANDY_DISABLE_SENTRY:-}"

  if ! ensure_sentry_api_connectivity "$mode_name"; then
    return
  fi

  notes="$(append_note "$notes" "sentry-cli info succeeded")"

  if [ "$SEND_TEST_EVENT" -eq 1 ]; then
    if [ -z "$dsn_value" ]; then
      record_result "$mode_name" "FAIL" \
        "Synthetic smoke event requested but SENTRY_DSN is missing." \
        "Smoke send requires SENTRY_DSN in env or .env." \
        "Set SENTRY_DSN then rerun: $(build_command_for_mode "$mode_name") --send-test-event"
      return
    fi

    if is_truthy "$disable_value"; then
      record_result "$mode_name" "FAIL" \
        "Synthetic smoke event requested but HANDY_DISABLE_SENTRY is enabled." \
        "Kill-switch prevents smoke event delivery." \
        "Unset HANDY_DISABLE_SENTRY or set 0, then rerun with --send-test-event"
      return
    fi

    local env_name="${SENTRY_ENVIRONMENT:-development}"
    run_cmd_capture env SENTRY_DSN="$dsn_value" sentry-cli send-event \
      --no-environ \
      -l error \
      -E "$env_name" \
      -m "sentry-monitoring smoke test" \
      -t "source:sentry-monitoring" \
      -t "mode:local-smoke"

    if [ "$CMD_RC" -ne 0 ]; then
      if is_network_error "$CMD_OUTPUT"; then
        api_network_failure_result "$mode_name" "Synthetic smoke event send"
        return
      fi

      if printf '%s' "$CMD_OUTPUT" | quiet_regex_match_stdin "dsn|DSN|data source name|missing dsn|could not parse dsn|invalid dsn"; then
        record_result "$mode_name" "FAIL" \
          "Synthetic smoke event send failed due to DSN configuration." \
          "SENTRY_DSN format appears invalid or missing." \
          "Validate SENTRY_DSN, then rerun with --send-test-event"
        return
      fi

      local reason
      reason="$(printf '%s' "$CMD_OUTPUT" | first_non_empty_line)"
      [ -z "$reason" ] && reason="unknown sentry-cli error"

      record_result "$mode_name" "FAIL" \
        "Synthetic smoke event send failed." \
        "$reason" \
        "sentry-cli send-event --log-level=debug"
      return
    fi

    notes="$(append_note "$notes" "synthetic event sent successfully")"
  fi

  record_result "$mode_name" "PASS" \
    "$notes" \
    "No immediate local smoke risk detected." \
    "If needed, rerun with --send-test-event for ingestion validation."
}

mode_prod_health() {
  local mode_name="prod-health"

  if ! prepare_api_context "$mode_name"; then
    return
  fi

  local release_value="$RELEASE_OVERRIDE"
  if [ -z "$release_value" ]; then
    release_value="$(expected_release)"
  fi

  local unresolved_count high_severity_count release_regression_count

  if ! query_issue_count "$mode_name" "is:unresolved" 50; then
    if is_permission_error "$CMD_OUTPUT"; then
      api_auth_failure_result "$mode_name" "Unresolved issue query"
      return
    fi
    if is_network_error "$CMD_OUTPUT"; then
      api_network_failure_result "$mode_name" "Unresolved issue query"
      return
    fi
    record_result "$mode_name" "FAIL" \
      "Failed to query unresolved issues." \
      "Check org/project/token correctness and permissions." \
      "$(build_command_for_mode "$mode_name") --strict-network"
    return
  fi
  unresolved_count="$QUERY_COUNT"

  if query_issue_count "$mode_name" "is:unresolved level:error" 50; then
    high_severity_count="$QUERY_COUNT"
  else
    high_severity_count="query_failed"
  fi

  if [ "$release_value" = "unknown" ]; then
    release_regression_count="release_unknown"
  else
    if query_issue_count "$mode_name" "is:unresolved release:${release_value}" 50; then
      release_regression_count="$QUERY_COUNT"
    else
      release_regression_count="query_failed"
    fi
  fi

  METRIC_UNRESOLVED_COUNT="$unresolved_count"
  METRIC_HIGH_SEVERITY_COUNT="$high_severity_count"
  METRIC_RELEASE_REGRESSION_COUNT="$release_regression_count"

  local evidence risks next
  evidence="unresolved_count=${unresolved_count}; high_severity_count=${high_severity_count}; release_regression_count=${release_regression_count}; org=${ORG}; project=${PROJECT}"
  if [ -n "$AUTO_RECOVERY_NOTES" ]; then
    evidence="$(append_note "$evidence" "$AUTO_RECOVERY_NOTES")"
  fi

  if [ "$unresolved_count" -eq 0 ]; then
    risks="No unresolved issues detected at query time."
    next="$(build_command_for_mode issue-triage)"
    record_result "$mode_name" "PASS" "$evidence" "$risks" "$next"
    return
  fi

  risks="Unresolved issues exist and require triage prioritization."
  next="$(build_command_for_mode issue-triage)"
  record_result "$mode_name" "PARTIAL" "$evidence" "$risks" "$next"
}

mode_release_gate() {
  local mode_name="release-gate"

  local tauri_ver cargo_ver pkg_ver
  tauri_ver="$(extract_first_version "$REPO_ROOT/src-tauri/tauri.conf.json" '"version"[[:space:]]*:[[:space:]]*"[^"]+')"
  cargo_ver="$(extract_cargo_version "$REPO_ROOT/src-tauri/Cargo.toml")"
  pkg_ver="$(extract_first_version "$REPO_ROOT/package.json" '"version"[[:space:]]*:[[:space:]]*"[^"]+')"

  local failed=0
  local notes=""
  local codictate_layout=0

  if [ -f "$REPO_ROOT/src-tauri/src/lib.rs" ] && [ -f "$REPO_ROOT/vite.config.ts" ] && [ -f "$REPO_ROOT/.github/workflows/build.yml" ]; then
    codictate_layout=1
  fi

  if [ "$codictate_layout" -eq 1 ]; then
    if [ -n "$tauri_ver" ] && [ -n "$cargo_ver" ] && [ "$tauri_ver" != "$cargo_ver" ]; then
      failed=1
      notes="$(append_note "$notes" "Version mismatch tauri.conf.json=${tauri_ver} Cargo.toml=${cargo_ver}")"
    fi
    if [ -n "$pkg_ver" ] && [ -n "$tauri_ver" ] && [ "$pkg_ver" != "$tauri_ver" ]; then
      failed=1
      notes="$(append_note "$notes" "Version mismatch package.json=${pkg_ver} tauri.conf.json=${tauri_ver}")"
    fi

    if ! check_fixed_string "$REPO_ROOT/src-tauri/src/lib.rs" 'format!("codictate@{}", env!("CARGO_PKG_VERSION"))'; then
      failed=1
      notes="$(append_note "$notes" "Runtime release fallback marker missing in src-tauri/src/lib.rs")"
    fi
    if ! check_fixed_string "$REPO_ROOT/vite.config.ts" 'process.env.SENTRY_RELEASE || `codictate@${packageJson.version}`'; then
      failed=1
      notes="$(append_note "$notes" "Vite release naming marker missing in vite.config.ts")"
    fi
    if ! check_fixed_string "$REPO_ROOT/.github/workflows/build.yml" 'SENTRY_RELEASE: codictate@${{ steps.get-version.outputs.version }}'; then
      failed=1
      notes="$(append_note "$notes" "CI release naming marker missing in .github/workflows/build.yml")"
    fi
    if ! check_fixed_string "$REPO_ROOT/.github/workflows/build.yml" 'SENTRY_ENVIRONMENT'; then
      failed=1
      notes="$(append_note "$notes" "CI environment marker missing in .github/workflows/build.yml")"
    fi
  else
    notes="$(append_note "$notes" "Generic repo mode: Codictate-specific release marker checks skipped")"
  fi

  local release_value="$RELEASE_OVERRIDE"
  if [ -z "$release_value" ]; then
    release_value="$(expected_release)"
  fi

  if [ "$release_value" = "unknown" ]; then
    failed=1
    notes="$(append_note "$notes" "Release value is unknown. Set SENTRY_RELEASE or pass --release")"
  fi

  if ! prepare_api_context "$mode_name"; then
    return
  fi

  run_cmd_capture sentry-cli releases info "$release_value" -o "$ORG" -p "$PROJECT"
  if [ "$CMD_RC" -ne 0 ]; then
    if is_network_error "$CMD_OUTPUT"; then
      api_network_failure_result "$mode_name" "Release visibility lookup"
      return
    fi
    if is_permission_error "$CMD_OUTPUT"; then
      api_auth_failure_result "$mode_name" "Release visibility lookup"
      return
    fi
    if is_project_not_found_error "$CMD_OUTPUT" || is_org_not_found_error "$CMD_OUTPUT"; then
      record_result "$mode_name" "FAIL" \
        "Release visibility lookup failed due to invalid org/project slug." \
        "Confirm slugs or enable auto discovery." \
        "$(build_command_for_mode "$mode_name") --auto-slug-discovery"
      return
    fi
    record_result "$mode_name" "FAIL" \
      "Release '$release_value' is not visible via sentry-cli releases info." \
      "Release upload may be missing, or release naming is mismatched." \
      "$(build_command_for_mode "$mode_name") --release $release_value"
    return
  fi

  local evidence risks next
  evidence="release=${release_value}; org=${ORG}; project=${PROJECT}"
  if [ -n "$AUTO_RECOVERY_NOTES" ]; then
    evidence="$(append_note "$evidence" "$AUTO_RECOVERY_NOTES")"
  fi

  if [ "$failed" -eq 1 ]; then
    risks="$notes"
    next="Fix release/version mismatches, then rerun $(build_command_for_mode "$mode_name")"
    record_result "$mode_name" "FAIL" "$evidence" "$risks" "$next"
    return
  fi

  risks="Release naming and visibility checks passed."
  next="Proceed to prod-health and issue-triage checks."
  record_result "$mode_name" "PASS" "$evidence" "$risks" "$next"
}

mode_issue_triage() {
  local mode_name="issue-triage"

  if ! prepare_api_context "$mode_name"; then
    return
  fi

  local unresolved_count new_last_day_count urgent_count

  if ! query_issue_count "$mode_name" "is:unresolved" 100; then
    if is_permission_error "$CMD_OUTPUT"; then
      api_auth_failure_result "$mode_name" "Base unresolved triage query"
      return
    fi
    if is_network_error "$CMD_OUTPUT"; then
      api_network_failure_result "$mode_name" "Base unresolved triage query"
      return
    fi
    record_result "$mode_name" "FAIL" \
      "Could not pull unresolved issues." \
      "Check org/project/token and retry." \
      "$(build_command_for_mode "$mode_name")"
    return
  fi
  unresolved_count="$QUERY_COUNT"

  if query_issue_count "$mode_name" "is:unresolved age:-1d" 100; then
    new_last_day_count="$QUERY_COUNT"
  else
    new_last_day_count="query_failed"
  fi

  if query_issue_count "$mode_name" "is:unresolved level:error" 100; then
    urgent_count="$QUERY_COUNT"
  else
    urgent_count="query_failed"
  fi

  METRIC_NEW_LAST_DAY_COUNT="$new_last_day_count"

  local evidence risks next
  evidence="unresolved_count=${unresolved_count}; new_last_day=${new_last_day_count}; likely_urgent=${urgent_count}; org=${ORG}; project=${PROJECT}"
  if [ -n "$AUTO_RECOVERY_NOTES" ]; then
    evidence="$(append_note "$evidence" "$AUTO_RECOVERY_NOTES")"
  fi

  if [ "$unresolved_count" -eq 0 ]; then
    risks="No unresolved issues to triage right now."
    next="Run verify-loop on demand after next release or incident report."
    record_result "$mode_name" "PASS" "$evidence" "$risks" "$next"
    return
  fi

  risks="Unresolved issues present. Triage new -> urgent -> recurring in that order."
  next="Open Sentry Issues filtered by: is:unresolved age:-1d, then is:unresolved level:error"
  record_result "$mode_name" "PASS" "$evidence" "$risks" "$next"
}

mode_verify_loop() {
  local mode_name="verify-loop"
  local release_value
  local prev_checked prev_release prev_unresolved prev_high prev_reg
  local current_unresolved current_high current_reg
  local new_since_last=0
  local resolved_since_last=0
  local overall_status evidence risks next

  AUTO_RECOVERY_NOTES=""

  mode_setup_preflight
  mode_local_smoke
  mode_release_gate
  mode_prod_health
  mode_issue_triage

  release_value="$RELEASE_OVERRIDE"
  if [ -z "$release_value" ]; then
    release_value="$(expected_release)"
  fi

  current_unresolved="${METRIC_UNRESOLVED_COUNT:-}"
  current_high="${METRIC_HIGH_SEVERITY_COUNT:-}"
  current_reg="${METRIC_RELEASE_REGRESSION_COUNT:-}"

  if [ -z "$current_unresolved" ] || [ "$current_unresolved" = "query_failed" ]; then
    record_result "$mode_name" "PARTIAL" \
      "Verification loop completed but unresolved snapshot is unavailable." \
      "API phase likely inconclusive or failed before metrics collection." \
      "Rerun with host-network access: $(build_command_for_mode "$mode_name")"
    return
  fi

  resolve_state_file_if_needed

  {
    IFS= read -r prev_checked
    IFS= read -r prev_release
    IFS= read -r prev_unresolved
    IFS= read -r prev_high
    IFS= read -r prev_reg
  } < <(load_previous_state "$STATE_FILE")

  if [ -n "$prev_unresolved" ] && printf '%s' "$prev_unresolved" | grep -Eq '^[0-9]+$'; then
    if [ "$current_unresolved" -gt "$prev_unresolved" ]; then
      new_since_last=$(( current_unresolved - prev_unresolved ))
    else
      resolved_since_last=$(( prev_unresolved - current_unresolved ))
    fi
  fi

  local checked_at
  checked_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  if ! persist_state_snapshot "$STATE_FILE" "$checked_at" "$release_value" "$current_unresolved" "$current_high" "$current_reg"; then
    record_result "$mode_name" "PARTIAL" \
      "Verification loop computed metrics but could not persist checkpoint state." \
      "State file write failed; delta tracking will reset next run." \
      "Ensure write access to $(dirname "$STATE_FILE") and rerun verify-loop"
    return
  fi

  overall_status="$(infer_result_status)"
  if [ "$overall_status" = "FAIL" ]; then
    overall_status="FAIL"
  elif [ "$overall_status" = "PARTIAL" ]; then
    overall_status="PARTIAL"
  else
    overall_status="PASS"
  fi

  evidence="unresolved_count=${current_unresolved}; new_since_last=${new_since_last}; resolved_since_last=${resolved_since_last}; high_severity_snapshot=${current_high}; release_regression_snapshot=${current_reg}; state_file=${STATE_FILE}; checked_at=${checked_at}"
  if [ -n "$prev_checked" ]; then
    evidence="$(append_note "$evidence" "previous_check=${prev_checked}")"
  else
    evidence="$(append_note "$evidence" "previous_check=none (baseline established)")"
  fi
  if [ -n "$AUTO_RECOVERY_NOTES" ]; then
    evidence="$(append_note "$evidence" "$AUTO_RECOVERY_NOTES")"
  fi

  if [ "$overall_status" = "FAIL" ]; then
    risks="One or more component checks failed; investigate failed mode sections first."
    next="Rerun failed modes individually, then rerun verify-loop: $(build_command_for_mode "$mode_name")"
  elif [ "$overall_status" = "PARTIAL" ]; then
    risks="Loop is inconclusive in at least one component or unresolved backlog exists."
    next="Rerun with host-network if needed: $(build_command_for_mode "$mode_name")"
  else
    risks="No blocking failures detected in current loop run."
    next="Run on demand after config changes, releases, or incident reports."
  fi

  record_result "$mode_name" "$overall_status" "$evidence" "$risks" "$next"
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --mode)
        MODE="$2"
        shift 2
        ;;
      --strict-network)
        STRICT_NETWORK=1
        shift
        ;;
      --send-test-event)
        SEND_TEST_EVENT=1
        shift
        ;;
      --org)
        ORG="$2"
        shift 2
        ;;
      --project)
        PROJECT="$2"
        shift 2
        ;;
      --release)
        RELEASE_OVERRIDE="$2"
        shift 2
        ;;
      --repo-root)
        REPO_ROOT="$2"
        REPO_ROOT_FROM_ARG=1
        shift 2
        ;;
      --state-file)
        STATE_FILE="$2"
        shift 2
        ;;
      --require-host-network)
        REQUIRE_HOST_NETWORK=1
        shift
        ;;
      --auto-slug-discovery)
        AUTO_SLUG_DISCOVERY=1
        shift
        ;;
      --no-auto-slug-discovery)
        AUTO_SLUG_DISCOVERY=0
        shift
        ;;
      --json)
        JSON_OUTPUT=1
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown argument: $1" >&2
        usage >&2
        exit 2
        ;;
    esac
  done
}

run_mode() {
  case "$1" in
    setup-preflight) mode_setup_preflight ;;
    local-smoke) mode_local_smoke ;;
    prod-health) mode_prod_health ;;
    release-gate) mode_release_gate ;;
    issue-triage) mode_issue_triage ;;
    verify-loop) mode_verify_loop ;;
    all)
      mode_setup_preflight
      mode_local_smoke
      mode_release_gate
      mode_prod_health
      mode_issue_triage
      ;;
    *)
      echo "Invalid mode: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
}

emit_summary_text() {
  local any_fail=0
  local any_partial=0

  echo "Sentry Monitoring Summary"
  echo "Mode: $MODE"
  echo

  while IFS=$'\t' read -r mode status evidence risks next; do
    [ -z "$mode" ] && continue
    echo "[$mode]"
    echo "Status: $status"
    echo "Evidence: $evidence"
    echo "Risks: $risks"
    echo "Next commands: $next"
    echo

    if [ "$status" = "FAIL" ]; then
      any_fail=1
    elif [ "$status" = "PARTIAL" ]; then
      any_partial=1
    fi
  done <"$TMP_RESULTS"

  if [ "$any_fail" -eq 1 ]; then
    echo "Overall: FAIL"
    return 1
  fi
  if [ "$any_partial" -eq 1 ]; then
    echo "Overall: PARTIAL"
    return 0
  fi
  echo "Overall: PASS"
  return 0
}

emit_summary_json() {
  "$PYTHON_BIN" - "$TMP_RESULTS" "$MODE" <<'PY'
import json
import sys

path = sys.argv[1]
mode = sys.argv[2]

results = []
overall = "PASS"
with open(path, "r", encoding="utf-8") as fh:
    for raw in fh:
        raw = raw.rstrip("\n")
        if not raw:
            continue
        parts = raw.split("\t", 4)
        if len(parts) != 5:
            continue
        mode_name, status, evidence, risks, next_cmds = parts
        message = f"{status}: {evidence} | Risks: {risks} | Next: {next_cmds}"
        item = {
            "mode": mode_name,
            "status": status,
            "message": message,
            "evidence": evidence,
            "risks": risks,
            "next_commands": next_cmds,
        }
        results.append(item)
        if status == "FAIL":
            overall = "FAIL"
        elif status == "PARTIAL" and overall != "FAIL":
            overall = "PARTIAL"

print(json.dumps({"mode": mode, "overall_status": overall, "results": results}, indent=2))
PY
}

main() {
  parse_args "$@"

  if ! init_search_tool; then
    echo "Missing dependency: requires either rg (preferred) or grep." >&2
    exit 2
  fi

  if ! command_exists sentry-cli; then
    echo "Missing dependency: sentry-cli" >&2
    exit 2
  fi

  if ! init_python_tool; then
    echo "Missing dependency: python3 (preferred) or python (fallback)" >&2
    exit 2
  fi

  if [ "$REPO_ROOT_FROM_ARG" -ne 1 ]; then
    resolve_default_repo_root
  fi

  load_local_env_defaults

  run_mode "$MODE"

  if [ "$JSON_OUTPUT" -eq 1 ]; then
    emit_summary_json
    exit 0
  fi

  if emit_summary_text; then
    exit 0
  fi
  exit 1
}

main "$@"
