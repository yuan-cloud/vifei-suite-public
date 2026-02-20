# Public Repo Settings Checklist

Purpose: make repository presentation, discoverability, and trust signals match Vifei's deterministic-evidence positioning.

Use this checklist before flipping public and before each release.

## A. Repository metadata

- [ ] Repository description is one sentence aligned to README opening line.
- [ ] Homepage URL points to primary docs or demo landing page.
- [ ] Topics are set and curated.
Recommended baseline topics:
`rust`, `cli`, `tui`, `determinism`, `event-sourcing`, `observability`, `ai-agents`, `forensics`, `reproducibility`.
- [ ] Social preview image is uploaded and current with the latest product narrative.
Status note: Section A requires GitHub repository UI/admin actions and cannot be fully verified from local workspace only.

## B. Community health files

- [x] `README.md` is current and has runnable commands.
- [x] `CONTRIBUTING.md` reflects maintainer merge policy.
- [x] `SECURITY.md` defines private reporting path and response expectations.
- [x] `SUPPORT.md` defines issue/report quality bar.
- [x] Issue forms exist under `.github/ISSUE_TEMPLATE/` and route security reports away from public issues.

## C. Default settings and branch protection

- [ ] Default branch is correct and protected.
- [ ] Required checks include CI job(s) that enforce fmt, clippy, tests.
- [ ] Force pushes to default branch are disabled.
- [ ] Deletion of default branch is disabled.
- [ ] Linear history requirement is enabled if team prefers rebase/merge discipline.
Status note: CI workflows enforce fmt/clippy/tests in code, but branch-protection enforcement itself must be set in GitHub settings.

## D. Releases and trust surfaces

- [ ] Release tags follow stable format (`vX.Y.Z`).
- [ ] Release notes include:
  - [ ] user-visible behavior changes
  - [ ] determinism or export-safety implications
  - [ ] migration notes, if any
- [ ] Trust docs are up to date:
  - [x] `docs/RELEASE_PACKAGING_CHECKLIST.md`
  - [x] `docs/RELEASE_TRUST_VERIFICATION.md`

## E. CI visibility and evidence

- [ ] GitHub Actions is enabled for the default branch and PRs.
- [ ] CI artifacts needed for debugging are retained with sensible expiration.
- [x] PTY-sensitive checks are clearly labeled or gated to avoid false confidence.
- [x] Failing checks point to replay commands in logs where possible.
Status note: Workflow definitions are in-repo; organization/repo-level Actions policy and retention defaults must be confirmed in GitHub settings.

## F. Launch copy quality gate

- [x] README language is concrete and operator-first.
- [x] Claims are paired with "how to verify" commands.
- [x] Avoid hype phrasing not backed by evidence artifacts.
- [x] Public-facing terms remain consistent across README, release notes, and tags.

## G. Final flip checklist

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Latest release commit is pushed
- [x] Checklist reviewed and dated in release notes or operator log

Reviewed: 2026-02-18 (local workspace audit, HEAD `48d7a1f`)
