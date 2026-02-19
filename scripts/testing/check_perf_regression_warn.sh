#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_PATH="${1:-.tmp/full-confidence/perf/bench_tour_metrics.json}"
BASELINE_PATH="${2:-docs/testing/perf-baseline-candidate-v1.json}"
MAX_P50_INCREASE_PCT="${MAX_P50_INCREASE_PCT:-15}"
MAX_P95_INCREASE_PCT="${MAX_P95_INCREASE_PCT:-20}"
MAX_P99_INCREASE_PCT="${MAX_P99_INCREASE_PCT:-25}"

if [[ ! -f "$ARTIFACT_PATH" ]]; then
  echo "::warning title=PERF_WARN::missing perf artifact at $ARTIFACT_PATH"
  echo "replay: VIFEI_TOUR_BENCH_ARTIFACT=$ARTIFACT_PATH cargo run -q -p vifei-tour --bin bench_tour --release"
  exit 0
fi

if [[ ! -f "$BASELINE_PATH" ]]; then
  echo "::warning title=PERF_WARN::baseline candidate missing at $BASELINE_PATH; calibration continues (warn-only)"
  echo "replay: cp $ARTIFACT_PATH docs/testing/perf-baseline-candidate-v1.json"
  exit 0
fi

python3 - "$ARTIFACT_PATH" "$BASELINE_PATH" "$MAX_P50_INCREASE_PCT" "$MAX_P95_INCREASE_PCT" "$MAX_P99_INCREASE_PCT" <<'PY'
import json
import sys
from pathlib import Path

artifact_path = Path(sys.argv[1])
baseline_path = Path(sys.argv[2])
max_p50 = float(sys.argv[3])
max_p95 = float(sys.argv[4])
max_p99 = float(sys.argv[5])

artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
baseline = json.loads(baseline_path.read_text(encoding="utf-8"))

required = ["run_ms_p50", "run_ms_p95", "run_ms_p99"]
for key in required:
    if key not in artifact.get("stats", {}):
        print(f"::warning title=PERF_WARN::artifact missing stats.{key}")
        sys.exit(0)
    if key not in baseline.get("stats", {}):
        print(f"::warning title=PERF_WARN::baseline missing stats.{key}")
        sys.exit(0)

limits = {
    "run_ms_p50": max_p50,
    "run_ms_p95": max_p95,
    "run_ms_p99": max_p99,
}

warnings = []
for metric, max_increase in limits.items():
    cur = float(artifact["stats"][metric])
    base = float(baseline["stats"][metric])
    if base <= 0:
        continue
    delta_pct = ((cur - base) / base) * 100.0
    if delta_pct > max_increase:
        warnings.append((metric, cur, base, delta_pct, max_increase))

if warnings:
    print("::warning title=PERF_WARN::bench_tour regression candidate detected (warn-only calibration mode)")
    for metric, cur, base, delta_pct, max_increase in warnings:
        print(
            f"::warning title=PERF_WARN::{metric} current={cur:.2f} baseline={base:.2f} "
            f"delta={delta_pct:.2f}% threshold={max_increase:.2f}%"
        )
    print(
        "replay: VIFEI_TOUR_BENCH_ARTIFACT=.tmp/full-confidence/perf/bench_tour_metrics.json "
        "VIFEI_PERF_TREND_DIR=.tmp/full-confidence/perf/trends "
        "cargo run -q -p vifei-tour --bin bench_tour --release"
    )
else:
    print("PERF_WARN: no threshold exceedance against current baseline candidate")
PY

exit 0
