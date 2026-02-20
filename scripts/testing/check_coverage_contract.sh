#!/usr/bin/env bash
set -euo pipefail

MATRIX="docs/testing/coverage-matrix-v0.1.md"
FASTLANE_DOC="docs/testing/FASTLANE.md"
DEFER_REGISTER="docs/testing/defer-register-v0.1.json"

fail() {
  local section="$1"
  local message="$2"
  local replay="$3"
  echo "CONTRACT_FAIL[$section] $message"
  echo "replay: $replay"
  exit 1
}

for required in "$MATRIX" "$FASTLANE_DOC" "$DEFER_REGISTER"; do
  [[ -f "$required" ]] || fail "CC0-files" "required contract file missing: $required" "test -f $required"
done

rg -q '^## v0\.1 Completeness Contract \("full enough"\)$' "$MATRIX" || \
  fail "CC1-headings" "coverage matrix missing completeness contract heading" "rg -n '^## v0\\.1 Completeness Contract \\(\"full enough\"\\)$' $MATRIX"
rg -q '^## Invariant And Decision Coverage Ownership$' "$MATRIX" || \
  fail "CC1-headings" "coverage matrix missing invariant ownership heading" "rg -n '^## Invariant And Decision Coverage Ownership$' $MATRIX"
rg -q '^## What Fastlane Covers$' "$FASTLANE_DOC" || \
  fail "CC1-headings" "FASTLANE doc missing coverage section heading" "rg -n '^## What Fastlane Covers$' $FASTLANE_DOC"

mapfile -t referenced_paths < <(
  rg -o '`[^`]+`' "$MATRIX" \
    | tr -d '`' \
    | rg '^(docs/|scripts/|crates/|\.github/)' \
    | sort -u
)

missing=()
for p in "${referenced_paths[@]}"; do
  [[ -e "$p" ]] || missing+=("$p")
done

if [[ "${#missing[@]}" -gt 0 ]]; then
  printf 'CONTRACT_FAIL[CC2-paths] coverage matrix references missing paths:\n'
  for p in "${missing[@]}"; do
    printf '  - %s\n' "$p"
  done
  echo "replay: scripts/testing/check_coverage_contract.sh"
  exit 1
fi

echo "CONTRACT_OK[CC0-files]"
echo "CONTRACT_OK[CC1-headings]"
echo "CONTRACT_OK[CC2-paths]"
