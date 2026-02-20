#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

if [[ "$#" -lt 1 ]]; then
  echo "Usage: scripts/testing/check_media_hygiene.sh <path> [path...]" >&2
  echo "Scans media/demo outputs for secret-like content; fails non-zero on findings." >&2
  exit 2
fi

python3 scripts/testing/check_media_hygiene.py "$@"
