#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

out_dir="$(mktemp -d)"
trap 'rm -rf "$out_dir"' EXIT

scripts/demo/visual_showcase_cut.sh "$out_dir" >/dev/null

required=(
  "$out_dir/VISUAL_SHOWCASE_SUMMARY.txt"
)

for file in "${required[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "[visual-cut-contract] missing required file: $file" >&2
    exit 1
  fi
done

if ! rg -q '^visual_showcase_status=PASS$' "$out_dir/VISUAL_SHOWCASE_SUMMARY.txt"; then
  echo "[visual-cut-contract] summary missing PASS marker" >&2
  exit 1
fi

for asset in incident-lens-showcase.svg forensic-lens-showcase.svg truth-hud-showcase.svg incident-lens-narrow-72.svg; do
  if ! rg -q "^${asset}_blake2s=[0-9a-f]{64}$" "$out_dir/VISUAL_SHOWCASE_SUMMARY.txt"; then
    echo "[visual-cut-contract] summary missing hash for $asset" >&2
    exit 1
  fi
done

echo "[visual-cut-contract] PASS: visual showcase cut contract"
