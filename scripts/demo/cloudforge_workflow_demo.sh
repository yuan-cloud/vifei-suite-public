#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/.tmp/cloudforge-workflow-demo}"
FIXTURE="$ROOT_DIR/fixtures/metals-commercial-workflow.jsonl"
REFUSAL_FIXTURE="$ROOT_DIR/docs/assets/readme/sample-refusal-eventlog.jsonl"

mkdir -p "$OUT_DIR"

echo "[1/4] Run deterministic tour on metals workflow fixture"
cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/tour" >"$OUT_DIR/tour.stdout.log" 2>"$OUT_DIR/tour.stderr.log"

echo "[2/4] Read KPI proxy metrics"
python3 - "$OUT_DIR/tour/metrics.json" <<'PY'
import json, sys
p = sys.argv[1]
m = json.load(open(p, 'r', encoding='utf-8'))
print(f"event_count_total={m.get('event_count_total')}")
print(f"tier_a_drops={m.get('tier_a_drops')}")
print(f"queue_pressure={m.get('queue_pressure')}")
print(f"degradation_level_final={m.get('degradation_level_final')}")
print("kpi_proxy_time_to_first_quote=derive from timetravel.capture transition timestamps")
print("kpi_proxy_followup_sla_breach_rate=derive from cadence events and anomaly windows")
print("kpi_proxy_stage_leakage_rate=derive from missing transition spans in replay")
PY

echo "[3/4] Verify clean share-safe export succeeds (security posture)"
set +e
cargo run -p vifei-tui --bin vifei -- \
  export "$FIXTURE" --share-safe --output "$OUT_DIR/clean-bundle.tar.zst" \
  --refusal-report "$OUT_DIR/clean-refusal-report.json" >"$OUT_DIR/export-clean.stdout.log" 2>"$OUT_DIR/export-clean.stderr.log"
rc=$?
set -e
if [[ "$rc" -ne 0 ]]; then
  echo "FAIL: clean share-safe export failed (exit $rc)"
  exit "$rc"
fi

echo "[4/4] Verify fail-closed behavior on synthetic refusal fixture"
set +e
cargo run -p vifei-tui --bin vifei -- \
  export "$REFUSAL_FIXTURE" --share-safe --output "$OUT_DIR/refused-bundle.tar.zst" \
  --refusal-report "$OUT_DIR/refusal-report.json" >"$OUT_DIR/export-refusal.stdout.log" 2>"$OUT_DIR/export-refusal.stderr.log"
rc=$?
set -e
if [[ "$rc" -ne 3 ]]; then
  echo "FAIL: refusal path should exit 3, got $rc"
  exit 1
fi

echo "DONE: CloudForge-style demo artifacts in $OUT_DIR"
echo "  - $OUT_DIR/tour/metrics.json"
echo "  - $OUT_DIR/tour/viewmodel.hash"
echo "  - $OUT_DIR/clean-bundle.tar.zst"
echo "  - $OUT_DIR/refusal-report.json"
