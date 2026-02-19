#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.tmp/e2e/tui}"
REPORT_PATH="$OUT_DIR/pty-preflight.log"
PROBE_PATH="$OUT_DIR/pty-preflight.typescript"

mkdir -p "$OUT_DIR"

fail() {
  local reason_code="$1"
  local message="$2"
  printf '{"schema_version":"vifei-pty-preflight-v1","status":"fail","reason_code":"%s","reason":"%s","probe":"%s","replay":"script -qefc true %s"}\n' \
    "$reason_code" "$message" "$PROBE_PATH" "$PROBE_PATH" | tee "$REPORT_PATH" >&2
  exit 1
}

if ! command -v script >/dev/null 2>&1; then
  fail "PTY_SCRIPT_UNAVAILABLE" "util-linux 'script' command is unavailable; install util-linux and re-run"
fi

set +e
script -qefc "true" "$PROBE_PATH" >/dev/null 2>&1
rc=$?
set -e

if [[ "$rc" -ne 0 ]]; then
  fail "PTY_ALLOCATION_DENIED" "PTY preflight failed (script exit=$rc). Ensure pseudo-terminal allocation is permitted in this environment."
fi

printf '{"schema_version":"vifei-pty-preflight-v1","status":"pass","reason_code":"PTY_OK","reason":"PTY allocation probe succeeded","probe":"%s","replay":"script -qefc true %s"}\n' \
  "$PROBE_PATH" "$PROBE_PATH" | tee "$REPORT_PATH"
exit 0
