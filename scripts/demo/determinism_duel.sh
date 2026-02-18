#!/usr/bin/env bash
set -euo pipefail

# Determinism Duel demo for Panopticon.
# Runs two independent stress tours and fails if viewmodel hashes diverge.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${1:-/tmp/panopticon_determinism_duel}"
FIXTURE="${2:-fixtures/large-stress.jsonl}"

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR/a" "$OUT_DIR/b"

echo "[duel] run A"
cargo run -p panopticon-tui --bin panopticon -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/a" >/dev/null

echo "[duel] run B"
cargo run -p panopticon-tui --bin panopticon -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/b" >/dev/null

hash_a="$(<"$OUT_DIR/a/viewmodel.hash")"
hash_b="$(<"$OUT_DIR/b/viewmodel.hash")"

echo "[duel] hash A: $hash_a"
echo "[duel] hash B: $hash_b"

if [[ "$hash_a" != "$hash_b" ]]; then
  echo "[duel] FAIL: hash mismatch detected" >&2
  exit 1
fi

echo "[duel] PASS: hashes match"
echo "[duel] outputs: $OUT_DIR"
