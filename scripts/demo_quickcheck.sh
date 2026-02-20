#!/usr/bin/env bash
set -euo pipefail

# Deterministic demo quickcheck for Vifei v0.1.
# Runs the evidence moments used in launch/demo scripts.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${1:-/tmp/vifei_demo_run}"
export OUT_DIR
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR/export"

echo "[demo] CLI help"
set +e
cargo run -p vifei-tui --bin vifei -- --help > "$OUT_DIR/help.txt"
help_rc=$?
set -e
if [[ "$help_rc" -ne 0 && "$help_rc" -ne 2 ]]; then
  echo "[demo] ERROR: help command failed with unexpected exit code: $help_rc" >&2
  exit 1
fi

echo "[demo] stress tour"
cargo run -p vifei-tui --bin vifei -- \
  tour fixtures/large-stress.jsonl --stress --output-dir "$OUT_DIR/tour"

echo "[demo] trust challenge hash"
cat "$OUT_DIR/tour/viewmodel.hash" | tee "$OUT_DIR/viewmodel.hash"

python3 - <<'PY'
import json
import os
from pathlib import Path
base = Path(os.environ["OUT_DIR"])
metrics = json.loads((base / "tour" / "metrics.json").read_text())
print(f"tier_a_drops={metrics['tier_a_drops']}")
print(f"degradation_level_final={metrics['degradation_level_final']}")
PY

echo "[demo] clean export (expected success)"
cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export/bundle.tar.zst" \
  --refusal-report "$OUT_DIR/export/refusal-report.json" \
  > "$OUT_DIR/export-success.txt"

echo "[demo] refusal export (expected refusal)"
set +e
cargo run -p vifei-tui --bin vifei -- \
  export docs/assets/readme/sample-refusal-eventlog.jsonl \
  --share-safe \
  --output "$OUT_DIR/export/refusal-bundle.tar.zst" \
  --refusal-report "$OUT_DIR/export/refusal-report-refused.json" \
  > "$OUT_DIR/export-refused.txt" 2>&1
rc=$?
set -e
if [[ "$rc" -eq 0 ]]; then
  echo "[demo] ERROR: refusal path unexpectedly succeeded" >&2
  exit 1
fi

echo "[demo] media provenance manifest"
generated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cargo run -p vifei-tour --bin media_provenance -- \
  --output "$OUT_DIR/media-provenance.json" \
  --generated-at "$generated_at" \
  --base-dir "$OUT_DIR" \
  --asset "$OUT_DIR/help.txt::cargo run -p vifei-tui --bin vifei -- --help" \
  --asset "$OUT_DIR/tour/metrics.json::cargo run -p vifei-tui --bin vifei -- tour fixtures/large-stress.jsonl --stress --output-dir $OUT_DIR/tour" \
  --asset "$OUT_DIR/tour/viewmodel.hash::cargo run -p vifei-tui --bin vifei -- tour fixtures/large-stress.jsonl --stress --output-dir $OUT_DIR/tour" \
  --asset "$OUT_DIR/export-success.txt::cargo run -p vifei-tui --bin vifei -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output $OUT_DIR/export/bundle.tar.zst --refusal-report $OUT_DIR/export/refusal-report.json" \
  --asset "$OUT_DIR/export-refused.txt::cargo run -p vifei-tui --bin vifei -- export docs/assets/readme/sample-refusal-eventlog.jsonl --share-safe --output $OUT_DIR/export/refusal-bundle.tar.zst --refusal-report $OUT_DIR/export/refusal-report-refused.json" \
  --asset "$OUT_DIR/export/refusal-report-refused.json::cargo run -p vifei-tui --bin vifei -- export docs/assets/readme/sample-refusal-eventlog.jsonl --share-safe --output $OUT_DIR/export/refusal-bundle.tar.zst --refusal-report $OUT_DIR/export/refusal-report-refused.json"

echo "[demo] media hygiene scan"
scripts/testing/check_media_hygiene.sh "$OUT_DIR"

echo "[demo] done: outputs in $OUT_DIR"
