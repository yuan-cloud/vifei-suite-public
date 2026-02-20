#!/usr/bin/env bash
set -euo pipefail

# Verify packaged Vifei release artifacts.
#
# Usage:
#   scripts/verify_release_artifacts.sh dist

DIST_DIR="${1:-dist}"

if [[ ! -f "$DIST_DIR/sha256sums.txt" ]]; then
  echo "[verify] missing checksum manifest: $DIST_DIR/sha256sums.txt" >&2
  exit 1
fi

if [[ ! -f "$DIST_DIR/vifei" ]]; then
  echo "[verify] missing artifact: $DIST_DIR/vifei" >&2
  exit 1
fi

if [[ ! -f "$DIST_DIR/bench_tour" ]]; then
  echo "[verify] missing artifact: $DIST_DIR/bench_tour" >&2
  exit 1
fi

(
  cd "$DIST_DIR"
  sha256sum -c sha256sums.txt
)

echo "[verify] checksum verification passed"
