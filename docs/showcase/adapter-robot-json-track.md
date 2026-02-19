# Adapter Demo Track: Robot JSON

This track is optimized for agent tooling and automation.

Use `--json` to force machine-readable envelopes.

## Envelope contract (expected in every response)

Required keys:
- `schema_version`
- `ok`
- `code`
- `message`
- `suggestions`
- `exit_code`

Success responses include `data`.

## 1) Tour success envelope

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json tour fixtures/small-session.jsonl --stress --output-dir /tmp/vifei-robot-tour
```

Expected:
- process exit `0`
- `ok=true`
- `code="OK"`
- `data.output_dir`, `data.event_count`, `data.tier_a_drops`, `data.viewmodel_hash`

## 2) Compare no-diff envelope

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json compare \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  docs/assets/readme/sample-export-clean-eventlog.jsonl
```

Expected:
- process exit `0`
- `ok=true`
- `code="OK"`
- `data.status="NO_DIFF"`

## 3) Compare divergence envelope

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json compare \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  docs/assets/readme/sample-refusal-eventlog.jsonl
```

Expected:
- process exit `5`
- `ok=false`
- `code="DIFF_FOUND"`
- `data.delta.divergences` present

## 4) Share-safe refusal envelope

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json export docs/assets/readme/sample-refusal-eventlog.jsonl \
  --share-safe \
  --output /tmp/vifei-robot-export/refused.tar.zst \
  --refusal-report /tmp/vifei-robot-export/refusal-report.json
```

Expected:
- process exit `3`
- `ok=false`
- `code="EXPORT_REFUSED"`
- refusal report path guidance in `suggestions`

## 5) Invalid args envelope

```bash
cargo run -p vifei-tui --bin vifei -- --json bogus-subcommand
```

Expected:
- process exit `2`
- `ok=false`
- `code="INVALID_ARGS"`
- actionable `suggestions`

## 6) Not-found envelope

```bash
cargo run -p vifei-tui --bin vifei -- \
  --json export does-not-exist.jsonl --share-safe --output /tmp/out.tar.zst
```

Expected:
- process exit `1`
- `ok=false`
- `code="NOT_FOUND"`

## Exit code map

- `0`: `OK`
- `1`: `NOT_FOUND`
- `2`: `INVALID_ARGS`
- `3`: `EXPORT_REFUSED`
- `4`: `RUNTIME_ERROR`
- `5`: `DIFF_FOUND`

## Parser-repair note

When unambiguous normalization is applied (for example underscore long-flag repair), response may include `notes` with the normalization details.
