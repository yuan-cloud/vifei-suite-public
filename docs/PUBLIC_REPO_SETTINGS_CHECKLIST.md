# Public Repo Settings Checklist

Purpose: make repository presentation, discoverability, and trust signals match Panopticon's deterministic-evidence positioning.

Use this checklist before flipping public and before each release.

## A. Repository metadata

- [ ] Repository description is one sentence aligned to README opening line.
- [ ] Homepage URL points to primary docs or demo landing page.
- [ ] Topics are set and curated.
Recommended baseline topics:
`rust`, `cli`, `tui`, `determinism`, `event-sourcing`, `observability`, `ai-agents`, `forensics`, `reproducibility`.
- [ ] Social preview image is uploaded and current with the latest product narrative.

## B. Community health files

- [ ] `README.md` is current and has runnable commands.
- [ ] `CONTRIBUTING.md` reflects maintainer merge policy.
- [ ] `SECURITY.md` defines private reporting path and response expectations.
- [ ] `SUPPORT.md` defines issue/report quality bar.
- [ ] Issue forms exist under `.github/ISSUE_TEMPLATE/` and route security reports away from public issues.

## C. Default settings and branch protection

- [ ] Default branch is correct and protected.
- [ ] Required checks include CI job(s) that enforce fmt, clippy, tests.
- [ ] Force pushes to default branch are disabled.
- [ ] Deletion of default branch is disabled.
- [ ] Linear history requirement is enabled if team prefers rebase/merge discipline.

## D. Releases and trust surfaces

- [ ] Release tags follow stable format (`vX.Y.Z`).
- [ ] Release notes include:
  - [ ] user-visible behavior changes
  - [ ] determinism or export-safety implications
  - [ ] migration notes, if any
- [ ] Trust docs are up to date:
  - [ ] `docs/RELEASE_PACKAGING_CHECKLIST.md`
  - [ ] `docs/RELEASE_TRUST_VERIFICATION.md`

## E. CI visibility and evidence

- [ ] GitHub Actions is enabled for the default branch and PRs.
- [ ] CI artifacts needed for debugging are retained with sensible expiration.
- [ ] PTY-sensitive checks are clearly labeled or gated to avoid false confidence.
- [ ] Failing checks point to replay commands in logs where possible.

## F. Launch copy quality gate

- [ ] README language is concrete and operator-first.
- [ ] Claims are paired with "how to verify" commands.
- [ ] Avoid hype phrasing not backed by evidence artifacts.
- [ ] Public-facing terms remain consistent across README, release notes, and tags.

## G. Final flip checklist

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Latest release commit is pushed
- [ ] Checklist reviewed and dated in release notes or operator log
