#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/docs/assets/readme"

cd "$ROOT_DIR"

echo "[readme-assets] generating deterministic asset pack..."
cargo run -p vifei-tui --bin capture_readme_assets

required=(
  "incident-lens.txt"
  "incident-lens.svg"
  "incident-lens-showcase.txt"
  "incident-lens-showcase.svg"
  "incident-lens-narrow-72.txt"
  "incident-lens-narrow-72.svg"
  "forensic-lens.txt"
  "forensic-lens.svg"
  "forensic-lens-showcase.txt"
  "forensic-lens-showcase.svg"
  "truth-hud-degraded.txt"
  "truth-hud-degraded.svg"
  "truth-hud-showcase.txt"
  "truth-hud-showcase.svg"
  "export-refusal.txt"
  "artifacts-view.txt"
  "architecture.mmd"
  "sample-eventlog.jsonl"
  "sample-export-clean-eventlog.jsonl"
)

for rel in "${required[@]}"; do
  if [[ ! -f "$OUT_DIR/$rel" ]]; then
    echo "[readme-assets] missing required asset: $rel" >&2
    exit 1
  fi
done

echo "[readme-assets] asset pack ready at $OUT_DIR"
