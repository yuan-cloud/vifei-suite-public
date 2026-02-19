# Adapter Demo Track: Refusal and Safety

This track is optimized for security, governance, and compliance audiences.

Use `--json` so refusal and runtime envelopes are machine-verifiable.

## 1) Share-safe refusal proof

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json export docs/assets/readme/sample-refusal-eventlog.jsonl \
  --share-safe \
  --output /tmp/vifei-safety/refused.tar.zst \
  --refusal-report /tmp/vifei-safety/refusal-report.json
```

Expected:
- process exit `3`
- `ok=false`
- `code="EXPORT_REFUSED"`
- refusal report written at `/tmp/vifei-safety/refusal-report.json`

Inspect blocked fields:

```bash
jq '.blocked_items[] | {event_id, field_path, pattern}' /tmp/vifei-safety/refusal-report.json
```

## 2) Safety guard for unsafe export mode

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json export docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --output /tmp/vifei-safety/no-scan.tar.zst
```

Expected:
- process exit `2`
- `ok=false`
- `code="INVALID_ARGS"`
- `suggestions` include a `--share-safe` correction

## 3) Runtime safety failure envelope (no silent fallback)

```bash
printf 'not-a-dir\n' > /tmp/vifei-output-file
cargo run -p vifei-tui --bin vifei -- \
  --json incident-pack \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --output-dir /tmp/vifei-output-file
```

Expected:
- process exit `4`
- `ok=false`
- `code="RUNTIME_ERROR"`
- no success envelope, no placeholder artifact claim

## 4) Contract-level refusal radar lane

```bash
scripts/demo/refusal_radar.sh --fast
```

Expected:
- refusal radar prints blocked-item evidence and `PASS`
- generated refusal report remains structured and deterministic

