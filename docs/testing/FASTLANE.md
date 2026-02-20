# Fastlane Smoke Suite (Sub-5-Minute Target)

## Purpose
Provide one deterministic, fail-fast command for developer feedback and PR validation, while preserving clear escalation paths to full suites.

Command:

```bash
scripts/e2e/fastlane.sh
```

## Runtime Budget
- Target: `<= 300s` on baseline dev machine (`FASTLANE_MAX_SECONDS`, default `300`).
- If budget is exceeded, fastlane fails with explicit fallback commands.

## What Fastlane Covers
1. Core invariant smoke
- append-writer `commit_index` assignment
- reducer determinism
- projection hash determinism
- docs constitution guard

2. CLI smoke
- `vifei --help`
- share-safe export success path
- share-safe refusal path (`export refused` + `Likely cause` contract)

3. TUI smoke
- width-bucket modality assertion (`modality_validation`)
- minimal interactive TUI path (`tui_e2e_interactive`, preflight-skip aware)

4. Artifact existence checks
- deterministic README capture artifacts (incident/forensic/truth/refusal)

## Outputs
- `run.jsonl`: machine-readable stage log with stable schema
- `summary.txt`: concise human-readable pass/fail and triage pointers
- `cmd/*.stdout.log` and `cmd/*.stderr.log`: per-stage transcripts

Default output root: `.tmp/fastlane`

## CI Usage
PR default lane runs:

```bash
scripts/e2e/fastlane.sh
```

Merge/release gating is handled by the separate `full-confidence` lane
(`docs/testing/FULL_CONFIDENCE.md`).

Invocation policy for test-governance scripts:

- Prefer direct executable form (`scripts/testing/<name>`) over `bash <script>` to keep local and CI behavior aligned.
- Python governance scripts in `scripts/testing/` are shebang-executable and follow the same policy.

## Contract Enforcement (CI)
Fastlane and full-confidence CI lanes enforce contract sections with explicit tags and replay commands:

1. `FL0-artifacts` / `FL1-stages`
- Verifies `.tmp/fastlane/run.jsonl` and `.tmp/fastlane/summary.txt` exist.
- Verifies `.tmp/fastlane/pty-preflight.log` exists.
- Verifies required fastlane stages are present in `run.jsonl`:
  - `cli_export_refusal_report`
  - `tui_pty_preflight`
  - `tui_interactive_smoke`
  - `fastlane_total`
- Replay:
  - `scripts/e2e/fastlane.sh`

2. `CC0-files` / `CC1-headings` / `CC2-paths`
- Implemented by `scripts/testing/check_coverage_contract.sh`.
- Verifies required coverage docs exist, required contract headings are present, and path references from `docs/testing/coverage-matrix-v0.1.md` resolve.
- Replay:
  - `scripts/testing/check_coverage_contract.sh`

3. `FC1-cli`
- Verifies full-confidence CLI E2E output includes semantic contract stages:
  - `tour_metadata`
  - `export_refusal_report`
- Replay:
  - `OUT_DIR=.tmp/full-confidence/cli-e2e scripts/e2e/cli_e2e.sh`

## Mapping to Full-Suite Commands
- Formatting/lints/full tests:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- Full CLI E2E:
  - `scripts/e2e/cli_e2e.sh`
- Full interactive TUI E2E:
  - `cargo test -p vifei-tui --test tui_e2e_interactive -- --nocapture`

Use full suites when fastlane fails or when shipping/release-proof validation is required.
