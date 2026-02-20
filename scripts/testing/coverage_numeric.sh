#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "COVERAGE_FAIL cargo not found"
  exit 1
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "COVERAGE_FAIL cargo llvm-cov is unavailable"
  echo "replay: cargo install cargo-llvm-cov"
  exit 1
fi

OUT_DIR="${1:-.tmp/full-confidence/coverage/numeric}"
mkdir -p "$OUT_DIR"

SUMMARY_TXT="$OUT_DIR/summary.txt"
SUMMARY_JSON="$OUT_DIR/summary.json"
LCOV_FILE="$OUT_DIR/lcov.info"

echo "COVERAGE_INFO out_dir=$OUT_DIR"

# Clean profile data to avoid cross-run contamination.
cargo llvm-cov clean --workspace

cargo llvm-cov \
  --workspace \
  --all-targets \
  --summary-only \
  > "$SUMMARY_TXT"

cargo llvm-cov \
  --workspace \
  --all-targets \
  --json \
  --summary-only \
  > "$SUMMARY_JSON"

cargo llvm-cov \
  --workspace \
  --all-targets \
  --lcov \
  --output-path "$LCOV_FILE"

if [[ ! -s "$SUMMARY_TXT" ]]; then
  echo "COVERAGE_FAIL empty summary output"
  exit 1
fi

if [[ ! -s "$SUMMARY_JSON" ]]; then
  echo "COVERAGE_FAIL empty json summary output"
  exit 1
fi

if [[ ! -s "$LCOV_FILE" ]]; then
  echo "COVERAGE_FAIL empty lcov output"
  exit 1
fi

echo "COVERAGE_OK summary=$SUMMARY_TXT"
echo "COVERAGE_OK summary_json=$SUMMARY_JSON"
echo "COVERAGE_OK lcov=$LCOV_FILE"
