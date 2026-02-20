#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

bundle_dir="$(mktemp -d)"
trap 'rm -rf "$bundle_dir"' EXIT

scripts/demo/package_launch_bundle.sh "$bundle_dir" >/dev/null

required=(
  "$bundle_dir/COMMAND_TRANSCRIPT.txt"
  "$bundle_dir/COMMAND_ASSET_MAP.md"
  "$bundle_dir/REPLAY_NOTES.md"
  "$bundle_dir/bundle-index.json"
  "$bundle_dir/demo-proof/media-provenance.json"
  "$bundle_dir/demo-proof/tour/metrics.json"
  "$bundle_dir/trust-cut/TRUST_DEMO_SUMMARY.txt"
  "$bundle_dir/visual-cut/VISUAL_SHOWCASE_SUMMARY.txt"
  "$bundle_dir/assets/readme/incident-lens.txt"
)

for file in "${required[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "[bundle-contract] missing required file: $file" >&2
    exit 1
  fi
done

cargo run -p vifei-tour --bin media_provenance -- \
  --verify "$bundle_dir/demo-proof/media-provenance.json" \
  --base-dir "$bundle_dir/demo-proof" >/dev/null

scripts/testing/check_media_hygiene.sh "$bundle_dir/demo-proof" >/dev/null

echo "[bundle-contract] PASS: launch bundle contract"
