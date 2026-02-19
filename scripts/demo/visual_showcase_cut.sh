#!/usr/bin/env bash
set -euo pipefail

# Visual showcase cut (target: ~45-90s on warm build).
# Focus: showcase profile assets + desktop/narrow readability proofs.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${1:-/tmp/vifei_visual_cut}"
mkdir -p "$OUT_DIR"

run() {
  echo "[visual-cut] $*"
  "$@"
}

run scripts/demo/live_incident_wall.sh --fast "$OUT_DIR/wall" >/dev/null

required_assets=(
  "docs/assets/readme/incident-lens-showcase.svg"
  "docs/assets/readme/forensic-lens-showcase.svg"
  "docs/assets/readme/truth-hud-showcase.svg"
  "docs/assets/readme/incident-lens-narrow-72.svg"
)

for file in "${required_assets[@]}"; do
  if [[ ! -s "$file" ]]; then
    echo "[visual-cut] FAIL: missing or empty asset $file" >&2
    exit 1
  fi
done

required_truth_tokens=("Level:" "Agg:" "Pressure:" "Drops:" "Export:" "Version:")
for token in "${required_truth_tokens[@]}"; do
  if ! rg -q "$token" docs/assets/readme/truth-hud-showcase.txt; then
    echo "[visual-cut] FAIL: truth HUD token missing: $token" >&2
    exit 1
  fi
done

if ! rg -q "Action Now \\(Anomalies\\)" docs/assets/readme/incident-lens-narrow-72.txt; then
  echo "[visual-cut] FAIL: narrow incident proof missing anomaly action section" >&2
  exit 1
fi

if ! rg -q "Forensic controls:" docs/assets/readme/incident-lens-narrow-72.txt; then
  echo "[visual-cut] FAIL: narrow incident proof missing forensic control hints" >&2
  exit 1
fi

python3 - "$OUT_DIR/VISUAL_SHOWCASE_SUMMARY.txt" <<'PY'
import hashlib
import pathlib
import sys

summary = pathlib.Path(sys.argv[1])
assets = [
    pathlib.Path("docs/assets/readme/incident-lens-showcase.svg"),
    pathlib.Path("docs/assets/readme/forensic-lens-showcase.svg"),
    pathlib.Path("docs/assets/readme/truth-hud-showcase.svg"),
    pathlib.Path("docs/assets/readme/incident-lens-narrow-72.svg"),
]

def digest(path: pathlib.Path) -> str:
    return hashlib.blake2s(path.read_bytes()).hexdigest()

lines = ["visual_showcase_status=PASS"]
for path in assets:
    lines.append(f"{path.name}_blake2s={digest(path)}")
summary.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

cat "$OUT_DIR/VISUAL_SHOWCASE_SUMMARY.txt"
echo "[visual-cut] PASS: outputs in $OUT_DIR"
