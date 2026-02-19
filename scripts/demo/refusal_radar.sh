#!/usr/bin/env bash
set -euo pipefail

# Refusal Radar demo for Vifei.
# Demonstrates share-safe export refusal and surfaces blocked fields.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MODE="full"
if [[ "${1:-}" == "--fast" || "${1:-}" == "--full" ]]; then
  MODE="${1#--}"
  shift
fi

OUT_DIR="${1:-/tmp/vifei_refusal_radar}"
mkdir -p "$OUT_DIR"

EVENTLOG="docs/assets/readme/sample-refusal-eventlog.jsonl"
if [[ "$MODE" == "full" ]]; then
  scripts/refresh_readme_assets.sh >/dev/null
fi

REPORT="$OUT_DIR/refusal-report.json"
BUNDLE="$OUT_DIR/refusal-bundle.tar.zst"

echo "[radar] mode: $MODE"
echo "[radar] input: $EVENTLOG"
started_at="$(date +%s)"

set +e
cargo run -p vifei-tui --bin vifei -- \
  export "$EVENTLOG" \
  --share-safe \
  --output "$BUNDLE" \
  --refusal-report "$REPORT" \
  >"$OUT_DIR/export.log" 2>&1
rc=$?
set -e

if [[ "$rc" -eq 0 ]]; then
  echo "[radar] FAIL: export unexpectedly succeeded" >&2
  exit 1
fi

if [[ ! -f "$REPORT" ]]; then
  echo "[radar] FAIL: refusal report not generated" >&2
  exit 1
fi

python3 - "$REPORT" <<'PY'
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
blocked = report.get("blocked_items", [])
print(f"[radar] blocked_count: {len(blocked)}")
if blocked:
    first = blocked[0]
    print(f"[radar] first_event: {first.get('event_id')}")
    print(f"[radar] first_field: {first.get('field_path')}")
    print(f"[radar] first_pattern: {first.get('matched_pattern')}")
PY

ended_at="$(date +%s)"
duration="$((ended_at - started_at))"
echo "[radar] PASS: refusal detected as expected"
echo "[radar] duration_sec: $duration"
echo "[radar] outputs: $OUT_DIR"
