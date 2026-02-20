#!/usr/bin/env bash
set -euo pipefail

# Trust-first demo cut (target: ~45-60s on warm build).
# Focus: deterministic hash stability, Tier A drops, refusal semantics.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${1:-/tmp/vifei_trust_cut}"
FIXTURE="${2:-fixtures/small-session.jsonl}"

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

run() {
  echo "[trust-demo] $*"
  "$@"
}

run cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/tour-a" >/dev/null

run cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/tour-b" >/dev/null

HASH_A="$(cat "$OUT_DIR/tour-a/viewmodel.hash")"
HASH_B="$(cat "$OUT_DIR/tour-b/viewmodel.hash")"

if [[ "$HASH_A" != "$HASH_B" ]]; then
  echo "[trust-demo] FAIL: deterministic hash mismatch" >&2
  echo "[trust-demo] hash_a=$HASH_A" >&2
  echo "[trust-demo] hash_b=$HASH_B" >&2
  exit 1
fi

python3 - "$OUT_DIR" > "$OUT_DIR/metrics-summary.txt" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
metrics = json.loads((root / "tour-a" / "metrics.json").read_text(encoding="utf-8"))
print(f"tier_a_drops={metrics['tier_a_drops']}")
print(f"degradation_level_final={metrics['degradation_level_final']}")
PY

if ! rg -q '^tier_a_drops=0$' "$OUT_DIR/metrics-summary.txt"; then
  echo "[trust-demo] FAIL: tier_a_drops is non-zero" >&2
  cat "$OUT_DIR/metrics-summary.txt" >&2
  exit 1
fi

run cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export-clean.tar.zst" \
  --refusal-report "$OUT_DIR/refusal-clean.json" >/dev/null

set +e
cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-refusal-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export-refused.tar.zst" \
  --refusal-report "$OUT_DIR/refusal-refused.json" >/dev/null 2>&1
refusal_rc=$?
set -e

if [[ "$refusal_rc" -eq 0 ]]; then
  echo "[trust-demo] FAIL: unsafe export unexpectedly succeeded" >&2
  exit 1
fi

python3 - "$OUT_DIR/refusal-refused.json" > "$OUT_DIR/refusal-summary.txt" <<'PY'
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
blocked = report.get("blocked_items", [])
print(f"blocked_items={len(blocked)}")
if not blocked:
    raise SystemExit(1)
PY

cat > "$OUT_DIR/TRUST_DEMO_SUMMARY.txt" <<EOF_SUMMARY
trust_demo_status=PASS
fixture=$FIXTURE
deterministic_hash=$HASH_A
$(cat "$OUT_DIR/metrics-summary.txt")
$(cat "$OUT_DIR/refusal-summary.txt")
EOF_SUMMARY

cat "$OUT_DIR/TRUST_DEMO_SUMMARY.txt"
echo "[trust-demo] PASS: outputs in $OUT_DIR"
