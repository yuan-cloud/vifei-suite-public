# CLI E2E Scripts

## Script

- `scripts/e2e/cli_e2e.sh`: deterministic CLI end-to-end validation for help, non-TTY `view`, stress `tour`, release-mode Tour benchmark snapshot, clean `export`, and refusal `export`.
- `scripts/e2e/fastlane.sh`: sub-5-minute deterministic smoke lane (core invariants, CLI success/refusal, TUI modality/interactive smoke, artifact checks).
- `scripts/e2e/pty_preflight.sh`: explicit PTY capability check for CI/operator environments before interactive TUI E2E.
- `cargo test -p panopticon-tui --test tui_e2e_interactive`: PTY-backed interactive TUI E2E for lens toggle, forensic navigation, Truth HUD visibility, clean exit, and narrow-terminal profile.

## Run

```bash
scripts/e2e/cli_e2e.sh
scripts/e2e/fastlane.sh
```

Optional output path:

```bash
OUT_DIR=/tmp/panopticon-e2e scripts/e2e/cli_e2e.sh
```

Interactive TUI output path:

```bash
PANOPTICON_E2E_OUT=/tmp/panopticon-e2e/tui cargo test -p panopticon-tui --test tui_e2e_interactive
```

CI/operator PTY preflight:

```bash
OUT_DIR=.tmp/e2e/tui scripts/e2e/pty_preflight.sh
```

## Output Contract

The script writes:

- `run.jsonl`: machine-parseable event log with stable `run_id`, monotonic `seq`, stage, status, exit code, and transcript path.
- `summary.txt`: human-readable pass/fail summary with command and log file pointers.
- `summary.txt` includes explicit `replay:` hints for failed stages.
- `cmd/*.stdout.log` and `cmd/*.stderr.log`: per-step transcripts.
- `tour/*`: generated Tour artifacts.
- `export/*`: generated export artifacts.

The interactive TUI test writes:

- `<workspace-root>/.tmp/e2e/tui/*.typescript`: PTY transcripts (first failure and retry attempt transcripts preserved).
- `<workspace-root>/.tmp/e2e/tui/*.assertions.log`: one-line JSON assertion summary with schema `panopticon-tui-e2e-assert-v1`, status (`pass`/`fail`/`skip`), attempt count, transcript pointers (`first_failure_transcript`, `retry_transcript`, `final_transcript`), and validation details.
- `<workspace-root>/.tmp/e2e/tui/pty-preflight.log`: one-line JSON preflight status with schema `panopticon-pty-preflight-v1`, deterministic `reason_code`, and replay command.

## Failure Triage

1. Open `summary.txt` and locate the first `FAIL` stage.
2. Inspect `cmd/<stage>.stdout.log` and `cmd/<stage>.stderr.log`.
3. If failure is artifact-related, verify the referenced path exists in `run.jsonl`.
4. Re-run the failing command directly from the logged `replay:` line (preferred) or `cmd:` line.
5. If refusal behavior changed, compare stderr text for `export refused` and `Likely cause`.
