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
- `cargo test`
3. Coverage inventory snapshot
- `cargo test --workspace --all-targets -- --list`
- captures `docs/testing/coverage-matrix-v0.1.md`
- captures `docs/testing/defer-register-v0.1.json`
4. CLI E2E
- `scripts/e2e/cli_e2e.sh`
5. Interactive TUI E2E
- `cargo test -p panopticon-tui --test tui_e2e_interactive -- --nocapture`

## Artifacts
The lane uploads `full-confidence-<sha>` containing:
- `.tmp/full-confidence/logs/*.log`
- `.tmp/full-confidence/coverage/test-inventory.txt`
- `.tmp/full-confidence/coverage/coverage-matrix-v0.1.md`
- `.tmp/full-confidence/coverage/defer-register-v0.1.json`
- `.tmp/full-confidence/cli-e2e/*` (structured transcripts and summaries)

## Relationship to fastlane
- `fastlane`: PR default, quick deterministic smoke and direct local fallback commands.
- `full-confidence`: push/merge/release gate with full evidence retention.
