# Adapter Demo Track: Human CLI

This track is optimized for operators and docs readers who want readable terminal output.

Use `--human` for each command to force human-oriented output even in piped contexts.

## 1) View (interactive inspection)

```bash
cargo run -p vifei-tui --bin vifei -- \
  --human view docs/assets/readme/sample-eventlog.jsonl --profile showcase
```

Outcome:
- Opens Incident Lens with Truth HUD visible.
- `Tab` toggles Incident and Forensic Lens.
- `q` exits.

## 2) Tour (deterministic proof run)

```bash
cargo run -p vifei-tui --bin vifei -- \
  --human tour fixtures/large-stress.jsonl --stress --output-dir /tmp/vifei-human-tour
```

Outcome:
- Writes proof artifacts under `/tmp/vifei-human-tour`.
- Key files: `metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`.

## 3) Export (share-safe clean path)

```bash
cargo run -p vifei-tui --bin vifei -- \
  --human export docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --share-safe \
  --output /tmp/vifei-human-export/bundle.tar.zst \
  --refusal-report /tmp/vifei-human-export/refusal-report.json
```

Outcome:
- Produces bundle artifact for clean input.
- Produces refusal report metadata file for audit traceability.

## 4) Incident pack (one-command evidence pack)

```bash
cargo run -p vifei-tui --bin vifei -- \
  --human incident-pack \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --output-dir /tmp/vifei-human-incident-pack
```

Outcome:
- Writes deterministic evidence pack with compare/replay/export artifacts.
- Key files: `manifest.json`, `compare/delta.json`, `replay/left.replay.json`, `replay/right.replay.json`.

## Optional mixed-format incident pack

```bash
cargo run -p vifei-tui --bin vifei -- \
  --human incident-pack \
  fixtures/small-session.jsonl \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --left-format cassette \
  --right-format eventlog \
  --output-dir /tmp/vifei-human-incident-pack-mixed
```

Outcome:
- Same artifact structure with explicit input-format mapping.
- Useful when comparing adapter inputs against canonical eventlog runs.
