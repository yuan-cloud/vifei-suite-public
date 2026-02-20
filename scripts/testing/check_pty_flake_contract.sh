#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-.tmp/full-confidence}"
PREFLIGHT_LOG="$OUT_DIR/pty-preflight.log"
ASSERTIONS_GLOB="$OUT_DIR/tui-e2e/*.assertions.log"
MAX_RETRY_PASSES="${PTY_MAX_RETRY_PASSES:-1}"
FASTLANE_RUN_JSONL="$OUT_DIR/run.jsonl"
FASTLANE_SUMMARY="$OUT_DIR/summary.txt"

fail() {
  local section="$1"
  local message="$2"
  local replay="$3"
  echo "CONTRACT_FAIL[$section] $message"
  echo "replay: $replay"
  exit 1
}

usage_hint() {
  cat <<EOF
usage:
  scripts/testing/check_pty_flake_contract.sh <full-confidence-out-dir>

expected layout:
  <OUT_DIR>/pty-preflight.log
  <OUT_DIR>/tui-e2e/*.assertions.log

examples:
  scripts/testing/check_pty_flake_contract.sh .tmp/full-confidence
  PTY_MAX_RETRY_PASSES=1 scripts/testing/check_pty_flake_contract.sh .tmp/full-confidence
EOF
}

if [[ -f "$FASTLANE_RUN_JSONL" && -f "$FASTLANE_SUMMARY" ]]; then
  if rg -q '"run_id":"fastlane-v0\.1"' "$FASTLANE_RUN_JSONL"; then
    echo "CONTRACT_FAIL[PTY0-lane-scope] wrong lane directory: '$OUT_DIR' is a fastlane output bundle."
    echo "hint: this checker validates full-confidence PTY assertion logs only."
    echo "replay: scripts/testing/check_pty_flake_contract.sh .tmp/full-confidence"
    usage_hint
    exit 1
  fi
fi

if [[ ! -f "$PREFLIGHT_LOG" ]]; then
  if [[ -f "$FASTLANE_RUN_JSONL" && -f "$FASTLANE_SUMMARY" ]]; then
    echo "CONTRACT_FAIL[PTY0-preflight-log] wrong lane directory: '$OUT_DIR' looks like fastlane output."
    echo "hint: this checker is full-confidence only; fastlane uses run.jsonl/summary.txt and does not emit tui-e2e assertion logs."
    echo "replay: scripts/testing/check_pty_flake_contract.sh .tmp/full-confidence"
    usage_hint
    exit 1
  fi
  fail "PTY0-preflight-log" "missing PTY preflight log at $PREFLIGHT_LOG" "OUT_DIR=$OUT_DIR scripts/e2e/pty_preflight.sh"
fi

preflight_status="$(python3 - "$PREFLIGHT_LOG" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
line = path.read_text(encoding="utf-8").strip()
if not line:
    raise SystemExit("empty preflight log")
payload = json.loads(line)
if payload.get("schema_version") != "vifei-pty-preflight-v1":
    raise SystemExit("unexpected preflight schema version")
status = payload.get("status")
if status not in {"pass", "fail"}:
    raise SystemExit("preflight status must be pass|fail")
if not payload.get("reason_code"):
    raise SystemExit("preflight reason_code missing")
print(status)
PY
)" || fail "PTY0-preflight-schema" "invalid PTY preflight payload" "cat $PREFLIGHT_LOG"

if [[ "$preflight_status" != "pass" ]]; then
  echo "CONTRACT_OK[PTY0-capability-gated] preflight status=$preflight_status; PTY suite is capability-gated"
  echo "CONTRACT_OK[PTY1-flake-budget] skipped because PTY capability is unavailable"
  exit 0
fi

shopt -s nullglob
assertion_files=($ASSERTIONS_GLOB)
if [[ "${#assertion_files[@]}" -eq 0 ]]; then
  if [[ -f "$FASTLANE_RUN_JSONL" && -f "$FASTLANE_SUMMARY" ]]; then
    echo "CONTRACT_FAIL[PTY1-assertion-files] wrong lane directory: '$OUT_DIR' appears to be fastlane output."
    echo "hint: fastlane is capability-gated but does not produce tui-e2e assertion logs."
    echo "replay: scripts/testing/check_pty_flake_contract.sh .tmp/full-confidence"
    usage_hint
    exit 1
  fi
  fail "PTY1-assertion-files" "preflight passed but no assertion logs were produced" "TERM=xterm-256color VIFEI_E2E_OUT=$OUT_DIR/tui-e2e cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture"
fi

python3 - "$MAX_RETRY_PASSES" "${assertion_files[@]}" <<'PY'
import json
import pathlib
import sys

max_retry_passes = int(sys.argv[1])
files = [pathlib.Path(p) for p in sys.argv[2:]]

retry_passes = 0
failed = []
skipped = []
seen = 0

for path in files:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("schema_version") != "vifei-tui-e2e-assert-v1":
        raise SystemExit(f"CONTRACT_FAIL[PTY1-assertion-schema] invalid schema in {path}")

    status = payload.get("status")
    attempt = payload.get("attempt")
    if status not in {"pass", "fail", "skip"}:
        raise SystemExit(f"CONTRACT_FAIL[PTY1-assertion-schema] invalid status in {path}")
    if not isinstance(attempt, int) or attempt < 0:
        raise SystemExit(f"CONTRACT_FAIL[PTY1-assertion-schema] invalid attempt in {path}")

    seen += 1
    if status == "fail":
        failed.append(path.name)
    elif status == "skip":
        skipped.append(path.name)
    elif attempt > 1:
        retry_passes += 1

if failed:
    print("CONTRACT_FAIL[PTY2-assertions] failing PTY assertion logs:")
    for name in failed:
        print(f"  - {name}")
    print("replay: TERM=xterm-256color VIFEI_E2E_OUT=.tmp/full-confidence/tui-e2e cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture")
    raise SystemExit(1)

if skipped:
    print("CONTRACT_FAIL[PTY2-assertions] unexpected skip assertion logs while preflight passed:")
    for name in skipped:
        print(f"  - {name}")
    print("replay: OUT_DIR=.tmp/full-confidence scripts/e2e/pty_preflight.sh && TERM=xterm-256color VIFEI_E2E_OUT=.tmp/full-confidence/tui-e2e cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture")
    raise SystemExit(1)

if retry_passes > max_retry_passes:
    print(
        f"CONTRACT_FAIL[PTY3-flake-budget] retry passes exceeded budget: {retry_passes} > {max_retry_passes}"
    )
    print(
        "replay: TERM=xterm-256color VIFEI_E2E_OUT=.tmp/full-confidence/tui-e2e cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture"
    )
    raise SystemExit(1)

print(f"CONTRACT_OK[PTY1-assertion-files] parsed={seen}")
print(f"CONTRACT_OK[PTY2-assertions] all PTY assertion logs passed without skips")
print(
    f"CONTRACT_OK[PTY3-flake-budget] retry_passes={retry_passes} max_retry_passes={max_retry_passes}"
)
PY
