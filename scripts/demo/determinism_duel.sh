#!/usr/bin/env bash
set -euo pipefail

# Determinism Duel demo for Vifei.
# Runs two independent stress tours and fails if viewmodel hashes diverge.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MODE="full"
if [[ "${1:-}" == "--fast" || "${1:-}" == "--full" ]]; then
  MODE="${1#--}"
  shift
fi

OUT_DIR="${1:-/tmp/vifei_determinism_duel}"
if [[ "$MODE" == "fast" ]]; then
  FIXTURE="fixtures/small-session.jsonl"
else
  FIXTURE="fixtures/large-stress.jsonl"
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR/a" "$OUT_DIR/b"

echo "[duel] mode: $MODE"
echo "[duel] fixture: $FIXTURE"
started_at="$(date +%s)"

echo "[duel] run A"
cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/a" >/dev/null

echo "[duel] run B"
cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/b" >/dev/null

hash_a="$(<"$OUT_DIR/a/viewmodel.hash")"
hash_b="$(<"$OUT_DIR/b/viewmodel.hash")"

echo "[duel] hash A: $hash_a"
echo "[duel] hash B: $hash_b"

if [[ "$hash_a" != "$hash_b" ]]; then
  echo "[duel] FAIL: hash mismatch detected" >&2
  exit 1
fi

ended_at="$(date +%s)"
duration="$((ended_at - started_at))"

echo "[duel] PASS: hashes match"
echo "[duel] duration_sec: $duration"
echo "[duel] outputs: $OUT_DIR"
