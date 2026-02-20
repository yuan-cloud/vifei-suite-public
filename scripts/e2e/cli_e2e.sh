#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.tmp/e2e-cli}"
RUN_ID="cli-e2e-v0.1"
LOG_JSONL="$OUT_DIR/run.jsonl"
SUMMARY_TXT="$OUT_DIR/summary.txt"

mkdir -p "$OUT_DIR"/{cmd,tour,export}
: > "$LOG_JSONL"
: > "$SUMMARY_TXT"

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

run_cmd() {
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
    return 1
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
    return 1
  fi
}

assert_contains() {
  local stage="$1"
  local path="$2"
  local pattern="$3"
  if command -v rg >/dev/null 2>&1; then
    rg -q "$pattern" "$path"
  else
    grep -Eq "$pattern" "$path"
  fi
  if [[ $? -eq 0 ]]; then
    log_json "info" "$stage" "ok" 0 "pattern '$pattern' found in $path" "$path"
    echo "[$stage] pattern ok: $pattern" >> "$SUMMARY_TXT"
  else
    log_json "error" "$stage" "failed" 1 "pattern '$pattern' missing in $path" "$path"
    echo "[$stage] pattern missing: $pattern in $path" >> "$SUMMARY_TXT"
    return 1
  fi
}

run_cmd quick_help_json 0 cargo run -p vifei-tui --bin vifei
assert_contains quick_help_json_stdout "$OUT_DIR/cmd/quick_help_json.stdout.log" "\"quick_help\""

run_cmd view_non_tty_refusal 4 \
  cargo run -p vifei-tui --bin vifei -- \
  view docs/assets/readme/sample-eventlog.jsonl

run_cmd tour_stress 0 \
  cargo run -p vifei-tui --bin vifei -- \
  tour fixtures/large-stress.jsonl --stress --output-dir "$OUT_DIR/tour"

assert_file tour_metrics "$OUT_DIR/tour/metrics.json"
assert_file tour_hash "$OUT_DIR/tour/viewmodel.hash"
assert_file tour_ansi "$OUT_DIR/tour/ansi.capture"
assert_file tour_timetravel "$OUT_DIR/tour/timetravel.capture"

python3 - \
  "$OUT_DIR/tour/metrics.json" \
  "$OUT_DIR/tour/viewmodel.hash" \
  "$OUT_DIR/tour/ansi.capture" \
  "$OUT_DIR/tour/timetravel.capture" \
  >> "$SUMMARY_TXT" <<'PY'
import json
import pathlib
import sys

metrics_path = pathlib.Path(sys.argv[1])
hash_path = pathlib.Path(sys.argv[2])
ansi_path = pathlib.Path(sys.argv[3])
timetravel_path = pathlib.Path(sys.argv[4])

metrics = json.loads(metrics_path.read_text())
timetravel = json.loads(timetravel_path.read_text())
vm_hash = hash_path.read_text().strip()
ansi = ansi_path.read_text()

drops = metrics.get("tier_a_drops")
event_count = metrics.get("event_count_total")
degradation_level_final = metrics.get("degradation_level_final")
queue_pressure = metrics.get("queue_pressure")
projection_version = metrics.get("projection_invariants_version")

print(
    "[tour_metrics] "
    f"tier_a_drops={drops} event_count_total={event_count} "
    f"degradation_level_final={degradation_level_final} queue_pressure={queue_pressure}"
)
if drops != 0:
    raise SystemExit("tier_a_drops must be 0")
if not isinstance(event_count, int) or event_count <= 0:
    raise SystemExit("event_count_total must be positive")
if not (isinstance(queue_pressure, float) or isinstance(queue_pressure, int)):
    raise SystemExit("queue_pressure must be numeric")
if queue_pressure < 0.0 or queue_pressure > 1.0:
    raise SystemExit("queue_pressure must be in [0.0, 1.0]")
if not isinstance(projection_version, str) or not projection_version:
    raise SystemExit("projection_invariants_version must be non-empty")

seek_points = timetravel.get("seek_points")
if not isinstance(seek_points, list) or not seek_points:
    raise SystemExit("timetravel.capture must contain non-empty seek_points")
if timetravel.get("projection_invariants_version") != projection_version:
    raise SystemExit("timetravel and metrics projection version mismatch")

last_seek = seek_points[-1]
if last_seek.get("commit_index") != event_count - 1:
    raise SystemExit("final seek commit_index must equal event_count_total - 1")
if last_seek.get("viewmodel_hash") != vm_hash:
    raise SystemExit("final seek viewmodel_hash must match viewmodel.hash artifact")
if vm_hash not in ansi:
    raise SystemExit("ansi.capture must include the final viewmodel hash")
PY
log_json "info" "tour_metrics" "ok" 0 "tier_a_drops validated as 0" "$OUT_DIR/tour/metrics.json"
tour_hash_value="$(tr -d '\n' < "$OUT_DIR/tour/viewmodel.hash")"
tour_event_count="$(python3 - "$OUT_DIR/tour/metrics.json" <<'PY'
import json
import sys
with open(sys.argv[1], "r", encoding="utf-8") as f:
    print(json.load(f)["event_count_total"])
PY
)"
log_json "info" "tour_metadata" "ok" 0 "viewmodel_hash=${tour_hash_value} event_count_total=${tour_event_count}" "$OUT_DIR/tour/metrics.json"

run_cmd tour_bench_release 0 \
  env VIFEI_TOUR_BENCH_ITERS="${VIFEI_TOUR_BENCH_ITERS:-3}" \
  cargo run -q -p vifei-tour --bin bench_tour --release
assert_contains tour_bench_release_stdout "$OUT_DIR/cmd/tour_bench_release.stdout.log" "tour_run_ms_p50="
assert_contains tour_bench_release_stdout "$OUT_DIR/cmd/tour_bench_release.stdout.log" "tour_run_ms_p95="
assert_contains tour_bench_release_stdout "$OUT_DIR/cmd/tour_bench_release.stdout.log" "tour_run_ms_p99="

run_cmd export_clean 0 \
  cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export/bundle.tar.zst" \
  --refusal-report "$OUT_DIR/export/refusal-clean.json"

assert_file export_bundle "$OUT_DIR/export/bundle.tar.zst"
assert_contains export_clean_stdout "$OUT_DIR/cmd/export_clean.stdout.log" "\"code\":\"OK\""

run_cmd export_refusal 3 \
  cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-refusal-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export/refused.tar.zst" \
  --refusal-report "$OUT_DIR/export/refusal-refused.json"

assert_contains export_refusal_stdout "$OUT_DIR/cmd/export_refusal.stdout.log" "\"code\":\"EXPORT_REFUSED\""
assert_contains export_refusal_stdout "$OUT_DIR/cmd/export_refusal.stdout.log" "\"blocked_items\""
assert_file export_refusal_report "$OUT_DIR/export/refusal-refused.json"

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

for item in blocked:
    if not item.get("event_id"):
        raise SystemExit("blocked item missing event_id")
    if not item.get("field_path"):
        raise SystemExit("blocked item missing field_path")
    if not item.get("matched_pattern"):
        raise SystemExit("blocked item missing matched_pattern")

for left, right in zip(blocked, blocked[1:]):
    l_key = (left["event_id"], left["field_path"], left["matched_pattern"])
    r_key = (right["event_id"], right["field_path"], right["matched_pattern"])
    if l_key > r_key:
        raise SystemExit("blocked_items are not stably sorted")

print(f"[export_refusal] blocked_items={len(blocked)} scanner_version={report.get('scanner_version')}")
PY
log_json "info" "export_refusal_report" "ok" 0 "refusal report schema/order validated" "$OUT_DIR/export/refusal-refused.json"

echo "E2E CLI run complete: $RUN_ID" | tee -a "$SUMMARY_TXT"
echo "Artifacts:"
echo "  JSONL log: $LOG_JSONL"
echo "  Summary:   $SUMMARY_TXT"
echo "  Captures:  $OUT_DIR"
