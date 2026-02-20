#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/.tmp/cloudforge-workflow-demo}"
FIXTURE="$ROOT_DIR/fixtures/metals-commercial-workflow.jsonl"
REFUSAL_FIXTURE="$ROOT_DIR/fixtures/metals-commercial-workflow-refusal.jsonl"

mkdir -p "$OUT_DIR"

echo "[1/4] Run deterministic tour on metals workflow fixture"
cargo run -p vifei-tui --bin vifei -- \
  tour "$FIXTURE" --stress --output-dir "$OUT_DIR/tour" >"$OUT_DIR/tour.stdout.log" 2>"$OUT_DIR/tour.stderr.log"

echo "[2/4] Read KPI proxy metrics"
python3 - "$OUT_DIR/tour/metrics.json" "$FIXTURE" "$OUT_DIR/kpi-proxy.json" <<'PY'
import json
import re
import sys

metrics_path, fixture_path, out_path = sys.argv[1:]
metrics = json.load(open(metrics_path, "r", encoding="utf-8"))
events = [json.loads(line) for line in open(fixture_path, "r", encoding="utf-8") if line.strip()]

first_rfq_intake = None
first_quote_commit = None
observed_types = []
followup_sla_breaches = 0
followup_windows = 0

for event in events:
    payload = event.get("payload", {})
    payload_type = payload.get("type")
    observed_types.append(payload_type)
    if payload_type == "ToolCall" and payload.get("tool") == "rfq.intake" and first_rfq_intake is None:
        first_rfq_intake = event.get("timestamp_ns")
    if payload_type == "ToolCall" and payload.get("tool") == "quote.commit" and first_quote_commit is None:
        first_quote_commit = event.get("timestamp_ns")
    if payload_type == "ToolResult" and payload.get("tool") == "outbound.cadence":
        followup_windows += 1
        result = payload.get("result", "")
        match = re.search(r"(\d+)h(\d+)m", result)
        if match:
            hours = int(match.group(1))
            minutes = int(match.group(2))
            total_minutes = hours * 60 + minutes
            # Demo SLA proxy: first response should arrive within 4 hours (240 minutes).
            if total_minutes > 240:
                followup_sla_breaches += 1

time_to_first_quote_hours = None
if first_rfq_intake is not None and first_quote_commit is not None and first_quote_commit >= first_rfq_intake:
    time_to_first_quote_hours = round((first_quote_commit - first_rfq_intake) / 3_600_000_000_000, 3)

expected_sequence = [
    "RunStart",
    "ToolCall",
    "ToolResult",
    "ToolCall",
    "ToolResult",
    "PolicyDecision",
    "ToolCall",
    "ToolResult",
    "ToolCall",
    "RunEnd",
]
matched = sum(1 for idx, expected in enumerate(expected_sequence) if idx < len(observed_types) and observed_types[idx] == expected)
stage_leakage_rate = round((len(expected_sequence) - matched) / len(expected_sequence), 3)
followup_sla_breach_rate = round(followup_sla_breaches / followup_windows, 3) if followup_windows else 0.0

summary = {
    "event_count_total": metrics.get("event_count_total"),
    "tier_a_drops": metrics.get("tier_a_drops"),
    "queue_pressure": metrics.get("queue_pressure"),
    "degradation_level_final": metrics.get("degradation_level_final"),
    "kpi_proxy_time_to_first_quote_hours": time_to_first_quote_hours,
    "kpi_proxy_followup_sla_breach_rate": followup_sla_breach_rate,
    "kpi_proxy_stage_leakage_rate": stage_leakage_rate,
}
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(summary, f, separators=(",", ":"), sort_keys=True)
    f.write("\n")

for key, value in summary.items():
    print(f"{key}={value}")
PY

echo "[3/4] Verify clean share-safe export succeeds (security posture)"
set +e
cargo run -p vifei-tui --bin vifei -- \
  export "$FIXTURE" --share-safe --output "$OUT_DIR/clean-bundle.tar.zst" \
  --refusal-report "$OUT_DIR/clean-refusal-report.json" >"$OUT_DIR/export-clean.stdout.log" 2>"$OUT_DIR/export-clean.stderr.log"
rc=$?
set -e
if [[ "$rc" -ne 0 ]]; then
  echo "FAIL: clean share-safe export failed (exit $rc)"
  exit "$rc"
fi

echo "[4/4] Verify fail-closed behavior on synthetic refusal fixture"
set +e
cargo run -p vifei-tui --bin vifei -- \
  export "$REFUSAL_FIXTURE" --share-safe --output "$OUT_DIR/refused-bundle.tar.zst" \
  --refusal-report "$OUT_DIR/refusal-report.json" >"$OUT_DIR/export-refusal.stdout.log" 2>"$OUT_DIR/export-refusal.stderr.log"
rc=$?
set -e
if [[ "$rc" -ne 3 ]]; then
  echo "FAIL: refusal path should exit 3, got $rc"
  exit 1
fi

echo "[4b/4] Print redaction proof from refusal report"
python3 - "$OUT_DIR/refusal-report.json" <<'PY'
import json
import sys

report = json.load(open(sys.argv[1], "r", encoding="utf-8"))
blocked = report.get("blocked_items", [])
print(f"blocked_items_count={len(blocked)}")
if blocked:
    first = blocked[0]
    print(f"first_matched_pattern={first.get('matched_pattern')}")
    print(f"first_redacted_match={first.get('redacted_match')}")
PY

echo "DONE: CloudForge-style demo artifacts in $OUT_DIR"
echo "  - $OUT_DIR/tour/metrics.json"
echo "  - $OUT_DIR/kpi-proxy.json"
echo "  - $OUT_DIR/tour/viewmodel.hash"
echo "  - $OUT_DIR/clean-bundle.tar.zst"
echo "  - $OUT_DIR/refusal-report.json"
