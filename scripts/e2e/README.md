# CLI E2E Scripts

## Script

- `scripts/e2e/cli_e2e.sh`: deterministic CLI end-to-end validation for help, non-TTY `view`, stress `tour`, clean `export`, and refusal `export`.
- `cargo test -p panopticon-tui --test tui_e2e_interactive`: PTY-backed interactive TUI E2E for lens toggle, forensic navigation, Truth HUD visibility, clean exit, and narrow-terminal profile.

## Run

```bash
scripts/e2e/cli_e2e.sh
```

Optional output path:

```bash
OUT_DIR=/tmp/panopticon-e2e scripts/e2e/cli_e2e.sh
```

Interactive TUI output path:

```bash
PANOPTICON_E2E_OUT=/tmp/panopticon-e2e/tui cargo test -p panopticon-tui --test tui_e2e_interactive
```

## Output Contract

The script writes:

- `run.jsonl`: machine-parseable event log with stable `run_id`, monotonic `seq`, stage, status, exit code, and transcript path.
- `summary.txt`: human-readable pass/fail summary with command and log file pointers.
- `cmd/*.stdout.log` and `cmd/*.stderr.log`: per-step transcripts.
- `tour/*`: generated Tour artifacts.
- `export/*`: generated export artifacts.

The interactive TUI test writes:

- `.tmp/e2e/tui/*.typescript`: PTY transcripts (first failure and retry attempt transcripts preserved).
- `.tmp/e2e/tui/*.assertions.log`: pass/fail summary with transcript pointers.

## Failure Triage

1. Open `summary.txt` and locate the first `FAIL` stage.
2. Inspect `cmd/<stage>.stdout.log` and `cmd/<stage>.stderr.log`.
3. If failure is artifact-related, verify the referenced path exists in `run.jsonl`.
4. Re-run the failing command directly from the logged `cmd:` line.
5. If refusal behavior changed, compare stderr text for `export refused` and `Likely cause`.
