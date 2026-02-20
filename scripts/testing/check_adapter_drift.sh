#!/usr/bin/env bash
set -euo pipefail

out_dir="${1:-.tmp/adapter-conformance}"
mkdir -p "$out_dir"

echo "Running adapter conformance drift gate..."
cargo test -p vifei-import --test adapter_conformance_drift -- --nocapture \
  2>&1 | tee "$out_dir/adapter-conformance-drift.log"

echo "Adapter conformance drift gate passed."
echo "Replay command: cargo test -p vifei-import --test adapter_conformance_drift -- --nocapture"
