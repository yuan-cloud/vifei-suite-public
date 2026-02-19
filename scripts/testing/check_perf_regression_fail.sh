#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_PATH="${1:-.tmp/full-confidence/perf/bench_tour_metrics.json}"
BASELINE_PATH="${2:-docs/testing/perf-baseline-lock-v1.json}"
MAX_P50_INCREASE_PCT="${MAX_P50_INCREASE_PCT:-15}"
MAX_P95_INCREASE_PCT="${MAX_P95_INCREASE_PCT:-20}"
MAX_P99_INCREASE_PCT="${MAX_P99_INCREASE_PCT:-25}"
MIN_BASELINE_ITERS="${MIN_BASELINE_ITERS:-10}"
MIN_ARTIFACT_ITERS="${MIN_ARTIFACT_ITERS:-5}"
OVERRIDE_FLAG="${VIFEI_PERF_GATE_OVERRIDE:-0}"
OVERRIDE_REASON="${VIFEI_PERF_GATE_OVERRIDE_REASON:-}"

fail() {
  local section="$1"
  local msg="$2"
  local replay="$3"
  echo "CONTRACT_FAIL[$section] $msg"
  echo "replay: $replay"
  exit 1
}

if [[ ! -f "$ARTIFACT_PATH" ]]; then
  fail \
    "PERF2-artifact" \
    "missing perf artifact at $ARTIFACT_PATH" \
    "VIFEI_TOUR_BENCH_ARTIFACT=$ARTIFACT_PATH cargo run -q -p vifei-tour --bin bench_tour --release"
fi

if [[ ! -f "$BASELINE_PATH" ]]; then
  fail \
    "PERF2-baseline" \
    "locked baseline missing at $BASELINE_PATH" \
    "cp $ARTIFACT_PATH $BASELINE_PATH"
fi

python3 - "$ARTIFACT_PATH" "$BASELINE_PATH" "$MAX_P50_INCREASE_PCT" "$MAX_P95_INCREASE_PCT" "$MAX_P99_INCREASE_PCT" "$MIN_BASELINE_ITERS" "$MIN_ARTIFACT_ITERS" <<'PY'
import json
import os
import sys
from pathlib import Path

artifact_path = Path(sys.argv[1])
baseline_path = Path(sys.argv[2])
max_p50 = float(sys.argv[3])
max_p95 = float(sys.argv[4])
max_p99 = float(sys.argv[5])
min_baseline_iters = int(sys.argv[6])
min_artifact_iters = int(sys.argv[7])

override_enabled = os.environ.get("VIFEI_PERF_GATE_OVERRIDE", "0") == "1"
override_reason = os.environ.get("VIFEI_PERF_GATE_OVERRIDE_REASON", "").strip()

artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
baseline = json.loads(baseline_path.read_text(encoding="utf-8"))

def fail(section: str, msg: str, replay: str) -> "None":
    print(f"CONTRACT_FAIL[{section}] {msg}")
    print(f"replay: {replay}")
    raise SystemExit(1)

if artifact.get("schema_version") != "vifei-tour-bench-v1":
    fail(
        "PERF2-schema",
        f"artifact schema mismatch: {artifact.get('schema_version')}",
        "cargo run -q -p vifei-tour --bin bench_tour --release",
    )
if baseline.get("schema_version") != "vifei-tour-bench-v1":
    fail(
        "PERF2-schema",
        f"baseline schema mismatch: {baseline.get('schema_version')}",
        f"cp {artifact_path} {baseline_path}",
    )

required = ["run_ms_p50", "run_ms_p95", "run_ms_p99", "iters"]
for key in required:
    if key not in artifact.get("stats", {}):
        fail("PERF2-shape", f"artifact missing stats.{key}", "regen bench artifact")
    if key not in baseline.get("stats", {}):
        fail("PERF2-shape", f"baseline missing stats.{key}", f"refresh {baseline_path}")

artifact_iters = int(artifact["stats"]["iters"])
baseline_iters = int(baseline["stats"]["iters"])
if artifact_iters < min_artifact_iters:
    fail(
        "PERF2-iters",
        f"artifact iters below minimum: {artifact_iters} < {min_artifact_iters}",
        "increase bench iterations before CI lock check",
    )
if baseline_iters < min_baseline_iters:
    fail(
        "PERF2-iters",
        f"baseline iters below lock minimum: {baseline_iters} < {min_baseline_iters}",
        f"refresh baseline: cp {artifact_path} {baseline_path}",
    )

limits = {
    "run_ms_p50": max_p50,
    "run_ms_p95": max_p95,
    "run_ms_p99": max_p99,
}

regressions = []
for metric, max_increase in limits.items():
    cur = float(artifact["stats"][metric])
    base = float(baseline["stats"][metric])
    if base <= 0:
        continue
    delta_pct = ((cur - base) / base) * 100.0
    if delta_pct > max_increase:
        regressions.append((metric, cur, base, delta_pct, max_increase))

if not regressions:
    print("PERF_GATE_OK no threshold exceedance against locked baseline")
    raise SystemExit(0)

if override_enabled:
    if not override_reason:
        fail(
            "PERF2-override",
            "override enabled but VIFEI_PERF_GATE_OVERRIDE_REASON is empty",
            "set VIFEI_PERF_GATE_OVERRIDE_REASON='<ticket-or-incident-id>'",
        )
    print(
        "::warning title=PERF_GATE_OVERRIDE::perf regression check overridden "
        f"(reason={override_reason})"
    )
    for metric, cur, base, delta_pct, max_increase in regressions:
        print(
            "::warning title=PERF_GATE_OVERRIDE::"
            f"{metric} current={cur:.2f} baseline={base:.2f} "
            f"delta={delta_pct:.2f}% threshold={max_increase:.2f}%"
        )
    print("PERF_GATE_OVERRIDE_APPLIED")
    raise SystemExit(0)

print("CONTRACT_FAIL[PERF2-regression] locked baseline threshold exceeded")
for metric, cur, base, delta_pct, max_increase in regressions:
    print(
        f"  - {metric}: current={cur:.2f} baseline={base:.2f} "
        f"delta={delta_pct:.2f}% threshold={max_increase:.2f}%"
    )
print(
    "replay: VIFEI_TOUR_BENCH_ARTIFACT=.tmp/full-confidence/perf/bench_tour_metrics.json "
    "VIFEI_PERF_TREND_DIR=.tmp/full-confidence/perf/trends "
    "cargo run -q -p vifei-tour --bin bench_tour --release"
)
print(
    "override (incident-only): VIFEI_PERF_GATE_OVERRIDE=1 "
    "VIFEI_PERF_GATE_OVERRIDE_REASON='<ticket>'"
)
raise SystemExit(1)
PY

exit 0
