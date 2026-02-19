#!/usr/bin/env bash
set -euo pipefail

# Record a reproducible showcase cast using asciinema.
# This captures an end-to-end trust/demo flow without introducing non-canonical artifacts.
#
# Usage:
#   scripts/capture_showcase_cast.sh [--fast|--full] [output_dir]
#
# Examples:
#   scripts/capture_showcase_cast.sh --fast /tmp/vifei-cast
#   scripts/capture_showcase_cast.sh --full

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="fast"
if [[ "${1:-}" == "--fast" || "${1:-}" == "--full" ]]; then
  MODE="${1#--}"
  shift
fi

OUT_DIR="${1:-/tmp/vifei-showcase-cast}"
mkdir -p "$OUT_DIR"

if ! command -v asciinema >/dev/null 2>&1; then
  echo "[cast] FAIL: asciinema is not installed." >&2
  echo "[cast] install: https://docs.asciinema.org/manual/installation/" >&2
  exit 1
fi

RUN_DIR="$OUT_DIR/demo-proof"
CAST_PATH="$OUT_DIR/vifei-showcase-${MODE}.cast"

echo "[cast] mode: $MODE"
echo "[cast] output_dir: $OUT_DIR"
echo "[cast] run_dir: $RUN_DIR"
echo "[cast] cast_path: $CAST_PATH"

cmd="scripts/demo_quickcheck.sh \"$RUN_DIR\""
if [[ "$MODE" == "full" ]]; then
  cmd="scripts/demo/determinism_duel.sh --full \"$OUT_DIR/duel-full\" && \
scripts/demo/refusal_radar.sh --full \"$OUT_DIR/radar-full\" && \
scripts/demo/live_incident_wall.sh --full \"$OUT_DIR/wall-full\""
fi

# -i 1 keeps captured timing readable and compact for social/demo playback.
asciinema rec -i 1 -c "$cmd" "$CAST_PATH"

echo "[cast] PASS: cast recorded"
echo "[cast] file: $CAST_PATH"
echo "[cast] replay: asciinema play $CAST_PATH"
