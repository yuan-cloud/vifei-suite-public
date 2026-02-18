# Release Readiness Gate · 2026-02-18

Purpose: final pre-public summary of what is complete, what is intentionally deferred, and what still requires manual GitHub admin action.

## Completed in-repo gates

- All tracked beads are closed in `.beads/issues.jsonl`.
- Quality gates are green in current workflow:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- Demo track is implemented and wired into CI smoke:
  - `scripts/demo/determinism_duel.sh` (`--fast` / `--full`)
  - `scripts/demo/refusal_radar.sh` (`--fast` / `--full`)
  - `scripts/demo/live_incident_wall.sh` (`--fast` / `--full`)
  - `scripts/testing/demo_smoke.sh`
- Public checklist updated in `docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md`.

## Intentionally deferred (tracked)

- Numeric line/function coverage reporting via `cargo llvm-cov` remains deferred.
- Canonical defer ledger entry:
  - `docs/testing/defer-register-v0.1.json`
  - id: `waiver-coverage-numeric-metrics`
  - status: `active`
  - revisit: `2026-02-24`
  - expires: `2026-03-31`

## Manual GitHub admin actions still required

These items cannot be proven from local repository state and must be confirmed in GitHub UI/org settings:

1. Repository metadata in Settings:
- description
- homepage URL
- topics
- social preview image

2. Branch protection and required checks:
- default branch protection enabled
- required checks enforced on PR merge
- force-push/deletion disabled as desired

3. Actions policy and artifact retention:
- Actions enabled for PR/default branch
- retention window configured to team preference

## Go/No-go statement

- Code and docs are in a strong state for public flip.
- No open implementation beads are blocking.
- Remaining blockers are administrative verification steps listed above.
