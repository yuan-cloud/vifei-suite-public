# CLI E2E Scripts

## Script

- `scripts/e2e/cli_e2e.sh`: deterministic CLI end-to-end validation for help, non-TTY `view`, stress `tour`, release-mode Tour benchmark snapshot, clean `export`, and refusal `export`.
- `scripts/e2e/fastlane.sh`: sub-5-minute deterministic smoke lane (core invariants, CLI success/refusal, TUI modality/interactive smoke, artifact checks).
- `scripts/e2e/pty_preflight.sh`: explicit PTY capability check for CI/operator environments before interactive TUI E2E.
- `cargo test -p vifei-tui --test tui_e2e_interactive`: PTY-backed interactive TUI E2E for lens toggle, forensic navigation, Truth HUD visibility, clean exit, and narrow-terminal profile.

## Run

```bash
scripts/e2e/cli_e2e.sh
scripts/e2e/fastlane.sh
```

Optional output path:

```bash
OUT_DIR=/tmp/vifei-e2e scripts/e2e/cli_e2e.sh
```

Interactive TUI output path:

```bash
VIFEI_E2E_OUT=/tmp/vifei-e2e/tui cargo test -p vifei-tui --test tui_e2e_interactive
```

CI/operator PTY preflight:

```bash
OUT_DIR=.tmp/e2e/tui scripts/e2e/pty_preflight.sh
```

## Output Contract

The script writes:

- `run.jsonl`: machine-parseable event log with stable `run_id`, monotonic `seq`, stage, status, exit code, and transcript path.
- `run.jsonl` includes deterministic metadata stages for trend comparison (for example Tour `viewmodel_hash` and `event_count_total`).
- `summary.txt`: human-readable pass/fail summary with command and log file pointers.
- `summary.txt` includes explicit `replay:` hints for failed stages.
- `cmd/*.stdout.log` and `cmd/*.stderr.log`: per-step transcripts.
- `tour/*`: generated Tour artifacts.
- `export/*`: generated export artifacts.

Artifact semantic checks enforced by `cli_e2e.sh`:

- Tour: validates `tier_a_drops == 0`, `queue_pressure` bounds, `event_count_total > 0`, projection invariants version parity between `metrics.json` and `timetravel.capture`, final seek-point commit/hash consistency, and hash presence in `ansi.capture`.
- Refusal export: validates `refusal-v0.1` schema shape, non-empty `blocked_items`, and deterministic blocked-item ordering by `(event_id, field_path, matched_pattern)`.

The interactive TUI test writes:

- `<workspace-root>/.tmp/e2e/tui/*.typescript`: PTY transcripts (first failure and retry attempt transcripts preserved).
- `<workspace-root>/.tmp/e2e/tui/*.assertions.log`: one-line JSON assertion summary with schema `vifei-tui-e2e-assert-v1`, status (`pass`/`fail`/`skip`), attempt count, transcript pointers (`first_failure_transcript`, `retry_transcript`, `final_transcript`), and validation details.
- `<workspace-root>/.tmp/e2e/tui/pty-preflight.log`: one-line JSON preflight status with schema `vifei-pty-preflight-v1`, deterministic `reason_code`, and replay command.

## Failure Triage

1. Open `summary.txt` and locate the first `FAIL` stage.
2. Inspect `cmd/<stage>.stdout.log` and `cmd/<stage>.stderr.log`.
3. If failure is artifact-related, verify the referenced path exists in `run.jsonl`.
4. Re-run the failing command directly from the logged `replay:` line (preferred) or `cmd:` line.
5. If refusal behavior changed, compare stderr text for `export refused` and `Likely cause`.
