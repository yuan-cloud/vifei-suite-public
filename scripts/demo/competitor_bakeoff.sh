#!/usr/bin/env bash
set -euo pipefail

# Competitor bakeoff harness for Vifei.
# Runs objective, reproducible checks for:
# 1) determinism stability
# 2) refusal semantics
# 3) explainability surfaces

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MODE="full"
if [[ "${1:-}" == "--fast" || "${1:-}" == "--full" ]]; then
  MODE="${1#--}"
  shift
fi

OUT_BASE="${1:-.tmp/competitor-bakeoff}"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_BASE/run-$RUN_ID"
mkdir -p "$OUT_DIR"

if [[ "$MODE" == "fast" ]]; then
  FIXTURE="fixtures/small-session.jsonl"
else
  FIXTURE="fixtures/large-stress.jsonl"
fi

echo "[bakeoff] mode: $MODE"
echo "[bakeoff] fixture: $FIXTURE"
echo "[bakeoff] out: $OUT_DIR"
started_at="$(date +%s)"

echo "[bakeoff] check 1/4: determinism duel"
scripts/demo/determinism_duel.sh "--$MODE" "$OUT_DIR/duel" \
  2>&1 | tee "$OUT_DIR/duel.log"

echo "[bakeoff] check 2/4: refusal radar"
scripts/demo/refusal_radar.sh "--$MODE" "$OUT_DIR/radar" \
  2>&1 | tee "$OUT_DIR/radar.log"

echo "[bakeoff] check 3/4: explainability capture"
cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/tour" >/dev/null

ANSI_CAPTURE="$OUT_DIR/tour/ansi.capture"
for token in "Level:" "Agg:" "Pressure:" "Drops:" "Export:" "Version:"; do
  if ! rg -q "$token" "$ANSI_CAPTURE"; then
    echo "[bakeoff] FAIL: explainability token missing: $token" >&2
    exit 1
  fi
done

echo "[bakeoff] check 4/4: incident evidence pack"
SAMPLE_EVENTLOG="docs/assets/readme/sample-export-clean-eventlog.jsonl"
cargo run -p vifei-tui --bin vifei -- \
  incident-pack "$SAMPLE_EVENTLOG" "$SAMPLE_EVENTLOG" \
  --output-dir "$OUT_DIR/incident-pack" >/dev/null

for required in \
  "$OUT_DIR/incident-pack/manifest.json" \
  "$OUT_DIR/incident-pack/compare/delta.json" \
  "$OUT_DIR/incident-pack/replay/left.replay.json" \
  "$OUT_DIR/incident-pack/replay/right.replay.json"
do
  if [[ ! -f "$required" ]]; then
    echo "[bakeoff] FAIL: missing incident-pack artifact $required" >&2
    exit 1
  fi
done

python3 - "$OUT_DIR/incident-pack/compare/delta.json" <<'PY'
import json
import sys
from pathlib import Path

delta = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
required = {
    "divergences",
    "left_event_count",
    "right_event_count",
    "left_run_id",
    "right_run_id",
}
if not required.issubset(delta.keys()):
    raise SystemExit("delta payload missing required keys")
if not isinstance(delta.get("divergences"), list):
    raise SystemExit("delta divergences must be a list")
PY

ended_at="$(date +%s)"
duration="$((ended_at - started_at))"

python3 - "$OUT_DIR" "$MODE" "$FIXTURE" "$duration" <<'PY'
import json
import sys
from pathlib import Path

out_dir = Path(sys.argv[1])
mode = sys.argv[2]
fixture = sys.argv[3]
duration = int(sys.argv[4])

duel_hash_a = (out_dir / "duel" / "a" / "viewmodel.hash").read_text(encoding="utf-8").strip()
duel_hash_b = (out_dir / "duel" / "b" / "viewmodel.hash").read_text(encoding="utf-8").strip()
refusal_report = json.loads((out_dir / "radar" / "refusal-report.json").read_text(encoding="utf-8"))
blocked_items = refusal_report.get("blocked_items", [])
report = {
    "schema_version": "vifei-competitor-bakeoff-v1",
    "mode": mode,
    "fixture": fixture,
    "duration_sec": duration,
    "checks": {
        "determinism_stability": {
            "pass": duel_hash_a == duel_hash_b,
            "hash_a": duel_hash_a,
            "hash_b": duel_hash_b,
        },
        "refusal_semantics": {
            "pass": len(blocked_items) > 0,
            "blocked_count": len(blocked_items),
        },
        "explainability_surface": {
            "pass": True,
            "tokens": ["Level:", "Agg:", "Pressure:", "Drops:", "Export:", "Version:"],
        },
        "incident_pack_artifacts": {
            "pass": True,
            "manifest": str(out_dir / "incident-pack" / "manifest.json"),
            "delta": str(out_dir / "incident-pack" / "compare" / "delta.json"),
        },
    },
}
(out_dir / "bakeoff-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print(f"[bakeoff] report: {out_dir / 'bakeoff-report.json'}")
PY

echo "[bakeoff] PASS: all proof checks succeeded"
