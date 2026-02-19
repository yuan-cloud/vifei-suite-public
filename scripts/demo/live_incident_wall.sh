#!/usr/bin/env bash
set -euo pipefail

# Live Incident Wall demo for Vifei.
# Produces premium showcase wall assets (incident/forensic/truth HUD) and prints paths.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MODE="full"
if [[ "${1:-}" == "--fast" || "${1:-}" == "--full" ]]; then
  MODE="${1#--}"
  shift
fi

OUT_DIR="${1:-docs/assets/readme}"
mkdir -p "$OUT_DIR"

echo "[wall] mode: $MODE"
started_at="$(date +%s)"

if [[ "$MODE" == "full" ]]; then
  scripts/refresh_readme_assets.sh >/dev/null
else
  cargo run -p vifei-tui --bin capture_readme_assets >/dev/null
fi

required=(
  "docs/assets/readme/incident-lens-showcase.svg"
  "docs/assets/readme/forensic-lens-showcase.svg"
  "docs/assets/readme/truth-hud-showcase.svg"
)

for file in "${required[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "[wall] FAIL: missing required asset $file" >&2
    exit 1
  fi
done

ended_at="$(date +%s)"
duration="$((ended_at - started_at))"

echo "[wall] PASS: showcase wall assets ready"
echo "[wall] duration_sec: $duration"
echo "[wall] incident: docs/assets/readme/incident-lens-showcase.svg"
echo "[wall] forensic: docs/assets/readme/forensic-lens-showcase.svg"
echo "[wall] truth_hud: docs/assets/readme/truth-hud-showcase.svg"
