#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

safe_file="$tmp_dir/safe.txt"
unsafe_file="$tmp_dir/unsafe.txt"
allow_file="$tmp_dir/allow.txt"

printf 'hello world\n' > "$safe_file"
printf 'token sk-THISISASYNTHETICTOKEN123456789\n' > "$unsafe_file"
printf 'openai_key\\|.*unsafe\\.txt.*sk-THISISASYNTHETICTOKEN123456789\n' > "$allow_file"

python3 scripts/testing/check_media_hygiene.py "$safe_file" >/dev/null

set +e
python3 scripts/testing/check_media_hygiene.py "$unsafe_file" >/dev/null 2>&1
rc_fail=$?
set -e
if [[ "$rc_fail" -eq 0 ]]; then
  echo "[contract] expected unsafe fixture to fail hygiene scan" >&2
  exit 1
fi

python3 scripts/testing/check_media_hygiene.py --allowlist "$allow_file" "$unsafe_file" >/dev/null

echo "[contract] PASS: hygiene scanner contract"
