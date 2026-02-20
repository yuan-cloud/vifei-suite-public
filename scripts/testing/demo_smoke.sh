#!/usr/bin/env bash
set -euo pipefail

# Fast smoke checks for showcase demo scripts.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_BASE="${1:-.tmp/demo-smoke}"
mkdir -p "$OUT_BASE"

echo "[smoke] determinism duel (fast)"
scripts/demo/determinism_duel.sh --fast "$OUT_BASE/duel"

echo "[smoke] refusal radar (fast)"
scripts/demo/refusal_radar.sh --fast "$OUT_BASE/radar"

echo "[smoke] live incident wall (fast)"
scripts/demo/live_incident_wall.sh --fast "$OUT_BASE/wall"

echo "[smoke] competitor bakeoff (fast)"
scripts/demo/competitor_bakeoff.sh --fast "$OUT_BASE/bakeoff"

echo "[smoke] PASS"
