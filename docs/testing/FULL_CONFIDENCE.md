# Full-Confidence CI Lane

## Purpose
`full-confidence` is the merge/release confidence lane. It runs broad validation and publishes actionable evidence artifacts for failures.

## Trigger
- GitHub Actions `CI` workflow on `push` events.
- This lane is required before `release-trust` can run.

## What it runs
1. Defer-register validation
2. Rust quality gates
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test` (interactive PTY tests excluded here and executed in dedicated lane step)
3. Coverage inventory snapshot
- `cargo test --workspace --all-targets -- --list`
- captures `docs/testing/coverage-matrix-v0.1.md`
- captures `docs/testing/defer-register-v0.1.json`
4. Numeric coverage report
- installs `cargo-llvm-cov` in CI
- runs `scripts/testing/coverage_numeric.sh`
- emits text summary, JSON summary, and `lcov.info`
5. CLI E2E
- `scripts/e2e/cli_e2e.sh`
6. PTY preflight
- `scripts/e2e/pty_preflight.sh`
 - capability probe only; emits deterministic reason codes and gates PTY-only checks by capability
6. Interactive TUI E2E
- `cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture`
- executed with explicit `TERM=xterm-256color` and workspace-scoped `VIFEI_E2E_OUT`
- runs only when preflight reports `status=pass`
- one bounded retry in CI to reduce transient PTY harness flake without masking persistent failures
- flake budget is enforced via `scripts/testing/check_pty_flake_contract.sh` (`PTY_MAX_RETRY_PASSES`, default `1`)
- lane scope note: `check_pty_flake_contract.sh` validates full-confidence outputs only; it should not be pointed at fastlane output directories

## Artifacts
The lane uploads `full-confidence-<sha>` containing:
- `.tmp/full-confidence/logs/*.log`
- `.tmp/full-confidence/pty-preflight.log`
- `.tmp/full-confidence/logs/pty-contract.log`
- `.tmp/full-confidence/coverage/test-inventory.txt`
- `.tmp/full-confidence/coverage/coverage-matrix-v0.1.md`
- `.tmp/full-confidence/coverage/defer-register-v0.1.json`
- `.tmp/full-confidence/coverage/numeric/summary.txt`
- `.tmp/full-confidence/coverage/numeric/summary.json`
- `.tmp/full-confidence/coverage/numeric/lcov.info`
- `.tmp/full-confidence/cli-e2e/*` (structured transcripts and summaries)

## Relationship to fastlane
- `fastlane`: PR default, quick deterministic smoke and direct local fallback commands.
- `full-confidence`: push/merge/release gate with full evidence retention.
