#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.tmp/fastlane}"
RUN_ID="fastlane-v0.1"
MAX_SECONDS="${FASTLANE_MAX_SECONDS:-300}"
LOG_JSONL="$OUT_DIR/run.jsonl"
SUMMARY_TXT="$OUT_DIR/summary.txt"

mkdir -p "$OUT_DIR"/{cmd,export}
: > "$LOG_JSONL"
: > "$SUMMARY_TXT"

START_TS="$(date +%s)"
SEQ=0

log_json() {
  local level="$1"
  local stage="$2"
  local status="$3"
  local exit_code="$4"
  local message="$5"
  local transcript="$6"
  SEQ=$((SEQ + 1))
  python3 - "$RUN_ID" "$SEQ" "$level" "$stage" "$status" "$exit_code" "$message" "$transcript" >> "$LOG_JSONL" <<'PY'
import json
import sys

run_id, seq, level, stage, status, exit_code, message, transcript = sys.argv[1:]
print(json.dumps({
    "run_id": run_id,
    "seq": int(seq),
    "level": level,
    "stage": stage,
    "status": status,
    "exit_code": int(exit_code),
    "message": message,
    "transcript": transcript,
}, separators=(",", ":"), sort_keys=True))
PY
}

stage_cmd() {
  local stage="$1"
  local expected="$2"
  shift 2
  local out_file="$OUT_DIR/cmd/${stage}.stdout.log"
  local err_file="$OUT_DIR/cmd/${stage}.stderr.log"
  local cmd_str
  cmd_str="$(printf '%q ' "$@")"
  cmd_str="${cmd_str% }"

  set +e
  "$@" >"$out_file" 2>"$err_file"
  local rc=$?
  set -e

  if [[ "$rc" -ne "$expected" ]]; then
    local replay_hint
    replay_hint="(cd $ROOT_DIR && $cmd_str)"
    log_json "error" "$stage" "failed" "$rc" "unexpected exit (expected $expected) cmd=$cmd_str" "$out_file"
    log_json "error" "$stage" "failed" "$rc" "replay_hint=$replay_hint" "$err_file"
    {
      echo "[$stage] FAIL expected=$expected actual=$rc"
      echo "  cmd: $cmd_str"
      echo "  stdout: $out_file"
      echo "  stderr: $err_file"
      echo "  replay: $replay_hint"
    } >> "$SUMMARY_TXT"
    exit 1
  fi

  log_json "info" "$stage" "ok" "$rc" "exit matched expected ($expected) cmd=$cmd_str" "$out_file"
  {
    echo "[$stage] PASS exit=$rc"
    echo "  cmd: $cmd_str"
    echo "  stdout: $out_file"
    echo "  stderr: $err_file"
  } >> "$SUMMARY_TXT"
}

assert_file() {
  local stage="$1"
  local path="$2"
  if [[ -f "$path" ]]; then
    log_json "info" "$stage" "ok" 0 "artifact present: $path" "$path"
    echo "[$stage] artifact ok: $path" >> "$SUMMARY_TXT"
  else
    log_json "error" "$stage" "failed" 1 "missing artifact: $path" "$path"
    echo "[$stage] missing artifact: $path" >> "$SUMMARY_TXT"
    exit 1
  fi
}

assert_contains() {
  local stage="$1"
  local path="$2"
  local pattern="$3"
  if rg -q "$pattern" "$path"; then
    log_json "info" "$stage" "ok" 0 "pattern '$pattern' found in $path" "$path"
    echo "[$stage] pattern ok: $pattern" >> "$SUMMARY_TXT"
  else
    log_json "error" "$stage" "failed" 1 "pattern '$pattern' missing in $path" "$path"
    echo "[$stage] pattern missing: $pattern in $path" >> "$SUMMARY_TXT"
    exit 1
  fi
}

cd "$ROOT_DIR"

# Core invariant smoke: canonical ordering + deterministic state/viewmodel hash paths.
stage_cmd core_commit_index 0 cargo test -p vifei-core eventlog::tests::commit_index_is_writer_assigned
stage_cmd core_reducer_determinism 0 cargo test -p vifei-core reducer::tests::determinism_10_runs
stage_cmd core_projection_hash 0 cargo test -p vifei-core projection::tests::test_viewmodel_hash_determinism
stage_cmd docs_guard 0 cargo test -p vifei-core --test docs_guard

# CLI smoke: help + share-safe success/refusal contracts.
stage_cmd cli_quick_help_json 0 cargo run -p vifei-tui --bin vifei
assert_contains cli_quick_help_json_stdout "$OUT_DIR/cmd/cli_quick_help_json.stdout.log" "\"quick_help\""

stage_cmd cli_export_clean 0 \
  cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export/bundle.tar.zst" \
  --refusal-report "$OUT_DIR/export/refusal-clean.json"
assert_file cli_export_bundle "$OUT_DIR/export/bundle.tar.zst"
assert_contains cli_export_clean_stdout "$OUT_DIR/cmd/cli_export_clean.stdout.log" "\"code\":\"OK\""

stage_cmd cli_export_refusal 3 \
  cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-refusal-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export/refused.tar.zst" \
  --refusal-report "$OUT_DIR/export/refusal-refused.json"
assert_contains cli_export_refusal_stdout "$OUT_DIR/cmd/cli_export_refusal.stdout.log" "\"code\":\"EXPORT_REFUSED\""
assert_contains cli_export_refusal_stdout "$OUT_DIR/cmd/cli_export_refusal.stdout.log" "\"blocked_items\""
assert_file cli_export_refusal_report "$OUT_DIR/export/refusal-refused.json"

python3 - "$OUT_DIR/export/refusal-refused.json" >> "$SUMMARY_TXT" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
report = json.loads(path.read_text())
blocked = report.get("blocked_items")
if report.get("report_version") != "refusal-v0.1":
    raise SystemExit("unexpected refusal report_version")
if not isinstance(blocked, list) or not blocked:
    raise SystemExit("refusal report blocked_items must be non-empty")

for left, right in zip(blocked, blocked[1:]):
    l_key = (left["event_id"], left["field_path"], left["matched_pattern"])
    r_key = (right["event_id"], right["field_path"], right["matched_pattern"])
    if l_key > r_key:
        raise SystemExit("blocked_items are not stably sorted")

print(f"[cli_export_refusal] blocked_items={len(blocked)}")
PY
log_json "info" "cli_export_refusal_report" "ok" 0 "refusal report schema/order validated" "$OUT_DIR/export/refusal-refused.json"

# Minimal TUI smoke: width bucket contracts + interactive PTY path (skip-aware by design).
stage_cmd tui_modality_smoke 0 \
  cargo test -p vifei-tui --test modality_validation width_buckets_preserve_required_surface_markers

set +e
env OUT_DIR="$OUT_DIR" scripts/e2e/pty_preflight.sh \
  > "$OUT_DIR/cmd/tui_pty_preflight.stdout.log" \
  2> "$OUT_DIR/cmd/tui_pty_preflight.stderr.log"
pty_rc=$?
set -e

assert_file tui_pty_preflight_log "$OUT_DIR/pty-preflight.log"
pty_status="$(python3 - "$OUT_DIR/pty-preflight.log" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
line = path.read_text(encoding="utf-8").strip()
payload = json.loads(line) if line else {}
status = payload.get("status")
print(status if status in {"pass", "fail"} else "invalid")
PY
)"

if [[ "$pty_status" == "invalid" ]]; then
  log_json "error" "tui_pty_preflight" "failed" 1 "invalid PTY preflight payload schema/status" "$OUT_DIR/pty-preflight.log"
  echo "[tui_pty_preflight] FAIL invalid PTY preflight payload" >> "$SUMMARY_TXT"
  exit 1
fi

if [[ "$pty_rc" -ne 0 && "$pty_status" != "fail" ]]; then
  log_json "error" "tui_pty_preflight" "failed" "$pty_rc" "preflight exited non-zero without status=fail" "$OUT_DIR/pty-preflight.log"
  echo "[tui_pty_preflight] FAIL inconsistent preflight exit/status rc=$pty_rc status=$pty_status" >> "$SUMMARY_TXT"
  exit 1
fi

log_json "info" "tui_pty_preflight" "ok" 0 "status=$pty_status rc=$pty_rc" "$OUT_DIR/pty-preflight.log"
echo "[tui_pty_preflight] status=$pty_status rc=$pty_rc" >> "$SUMMARY_TXT"

if [[ "$pty_status" == "pass" ]]; then
  stage_cmd tui_interactive_smoke 0 \
    cargo test -p vifei-tui --test tui_e2e_interactive interactive_tui_flow_lens_toggle_nav_and_quit -- --nocapture
else
  log_json "info" "tui_interactive_smoke" "ok" 0 "gated: PTY capability unavailable for runner" "$OUT_DIR/pty-preflight.log"
  echo "[tui_interactive_smoke] GATED due to PTY capability status=fail (see $OUT_DIR/pty-preflight.log)" >> "$SUMMARY_TXT"
fi

# Artifact existence smoke from deterministic README capture set.
assert_file readme_asset_incident docs/assets/readme/incident-lens.txt
assert_file readme_asset_incident_narrow docs/assets/readme/incident-lens-narrow-72.txt
assert_file readme_asset_forensic docs/assets/readme/forensic-lens.txt
assert_file readme_asset_truth_hud docs/assets/readme/truth-hud-degraded.txt
assert_file readme_asset_refusal docs/assets/readme/export-refusal.txt

ELAPSED="$(( $(date +%s) - START_TS ))"
log_json "info" "fastlane_total" "ok" 0 "elapsed_seconds=$ELAPSED max_seconds=$MAX_SECONDS" "$SUMMARY_TXT"
echo "[fastlane_total] elapsed_seconds=$ELAPSED max_seconds=$MAX_SECONDS" >> "$SUMMARY_TXT"

if [[ "$ELAPSED" -gt "$MAX_SECONDS" ]]; then
  log_json "error" "fastlane_budget" "failed" 1 "fastlane exceeded budget: ${ELAPSED}s > ${MAX_SECONDS}s" "$SUMMARY_TXT"
  {
    echo "[fastlane_budget] FAIL elapsed=${ELAPSED}s budget=${MAX_SECONDS}s"
    echo "  full-suite fallback:"
    echo "  - cargo fmt --check"
    echo "  - cargo clippy --all-targets -- -D warnings"
    echo "  - cargo test"
    echo "  - scripts/e2e/cli_e2e.sh"
    echo "  - cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture"
  } >> "$SUMMARY_TXT"
  exit 1
fi

{
  echo
  echo "Fastlane complete: $RUN_ID"
  echo "JSONL log: $LOG_JSONL"
  echo "Summary:   $SUMMARY_TXT"
  echo "Elapsed:   ${ELAPSED}s"
  echo
  echo "Full-suite fallback commands:"
  echo "  cargo fmt --check"
  echo "  cargo clippy --all-targets -- -D warnings"
  echo "  cargo test"
  echo "  scripts/e2e/cli_e2e.sh"
  echo "  cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture"
} | tee -a "$SUMMARY_TXT"
