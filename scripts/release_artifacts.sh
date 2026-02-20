#!/usr/bin/env bash
set -euo pipefail

# Build and package release artifacts for Vifei v0.1.
#
# Usage:
#   scripts/release_artifacts.sh
#   scripts/release_artifacts.sh dist

OUT_DIR="${1:-dist}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[release] building release binaries"
cargo build --release -p vifei-tui
cargo build --release -p vifei-tour

mkdir -p "$OUT_DIR"
cp target/release/vifei "$OUT_DIR/"
cp target/release/bench_tour "$OUT_DIR/"

# Deterministic checksum ordering for stable verification logs.
(
  cd "$OUT_DIR"
  LC_ALL=C ls -1 vifei bench_tour | xargs sha256sum > sha256sums.txt
)

echo "[release] wrote artifacts to $OUT_DIR"
echo "[release] checksum manifest: $OUT_DIR/sha256sums.txt"
