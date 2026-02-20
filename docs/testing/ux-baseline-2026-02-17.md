# UX Baseline Run 2026-02-17

## Environment
- Host: local dev sandbox
- Rust/Cargo: workspace default toolchain
- Terminal profiles: desktop=120x30, narrow=72x22
- PTY capability: unavailable in this environment (`script: failed to create pseudo-terminal: Permission denied`)

## Evidence Commands
```bash
OUT_DIR=.tmp/e2e-cli-baseline scripts/e2e/cli_e2e.sh
VIFEI_E2E_OUT=.tmp/e2e-tui-baseline cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture
```

## Task Results
| Task | Profile | Status | Completion (s) | Error type | Confidence (1-5) | Evidence |
|---|---|---|---:|---|---:|---|
| first_run_orientation | desktop | pass | 3 | none | 4 | `.tmp/e2e-cli-baseline/cmd/help.stdout.log` |
| incident_to_forensic_triage | desktop | skip | 0 | pty_env | 3 | `crates/vifei-tui/.tmp/e2e-tui-baseline/interactive_tui_flow_lens_toggle_nav_and_quit.assertions.log` |
| trust_verification | desktop | pass | 4 | none | 5 | `.tmp/e2e-cli-baseline/tour/metrics.json` |
| share_safe_refusal_recovery | desktop | pass | 2 | none | 5 | `.tmp/e2e-cli-baseline/cmd/export_refusal.stderr.log` |
| incident_to_forensic_triage | narrow | skip | 0 | pty_env | 3 | `crates/vifei-tui/.tmp/e2e-tui-baseline/interactive_tui_narrow_terminal_profile_stays_healthy.assertions.log` |

## Summary
- runnable_tasks: 3
- passed_tasks: 3
- pass_rate: 1.0
- confidence_avg: 4.0
- overall: GREEN with environment caveat (interactive PTY checks skipped due host constraints)

## Findings -> Beads
- [BUG] TUI e2e output path is crate-relative when run from integration tests, diverging from CLI e2e artifact root -> `bd-kko`
- [DOCS/CI] PTY preflight skip path needs explicit CI environment check/documented prerequisite (`script` PTY availability) -> `bd-1un`

## Notes
- No CLI contract regressions observed; refusal messaging and stress proof artifacts remain intact.
- PTY skip behavior is deterministic and logged with explicit reason.
