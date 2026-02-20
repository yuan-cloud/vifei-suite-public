#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

out_dir="$(mktemp -d)"
trap 'rm -rf "$out_dir"' EXIT

scripts/demo/trust_demo_cut.sh "$out_dir" fixtures/small-session.jsonl >/dev/null

required=(
  "$out_dir/TRUST_DEMO_SUMMARY.txt"
  "$out_dir/tour-a/metrics.json"
  "$out_dir/tour-a/viewmodel.hash"
  "$out_dir/tour-b/viewmodel.hash"
  "$out_dir/refusal-refused.json"
)

for file in "${required[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "[trust-demo-contract] missing required file: $file" >&2
    exit 1
  fi
done

if ! rg -q '^trust_demo_status=PASS$' "$out_dir/TRUST_DEMO_SUMMARY.txt"; then
  echo "[trust-demo-contract] summary missing PASS marker" >&2
  exit 1
fi

if ! rg -q '^tier_a_drops=0$' "$out_dir/TRUST_DEMO_SUMMARY.txt"; then
  echo "[trust-demo-contract] summary missing zero-drop proof" >&2
  exit 1
fi

if ! rg -q '^blocked_items=[1-9][0-9]*$' "$out_dir/TRUST_DEMO_SUMMARY.txt"; then
  echo "[trust-demo-contract] refusal summary missing blocked items" >&2
  exit 1
fi

echo "[trust-demo-contract] PASS: trust demo cut contract"
