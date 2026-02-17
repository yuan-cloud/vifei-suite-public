# Launch Communications Pack (v0.1)

Single-source communications pack for `bd-3qq.4`.

This pack intentionally excludes README authoring and uses only verified claims from:

- `docs/README_VERIFICATION.md`
- `docs/RELEASE_TRUST_VERIFICATION.md`
- `docs/RELEASE_PACKAGING_CHECKLIST.md`
- `docs/DEMO_SCRIPT.md`

## 1) GitHub Release Notes Template

Title:

`Panopticon Suite v0.1.0`

Body template:

```markdown
## Panopticon Suite v0.1.0

Panopticon is a deterministic, local-first flight recorder for AI agent runs.

### Highlights
- Deterministic stress Tour with proof artifacts (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`)
- Truth-first overload posture (Tier A truth preserved; projection degrades)
- Share-safe export behavior with explicit refusal reports when secrets are detected

### Verified In This Release
- README command flow and trust challenge validated (see `docs/README_VERIFICATION.md`)
- Release artifact build + checksum verification flow validated
- Release-trust CI job emits checksum manifest and provenance attestation

### Artifacts
- `panopticon`
- `bench_tour`
- `sha256sums.txt`

### Verification
```bash
scripts/verify_release_artifacts.sh dist
```

### Known Limits (v0.1)
- TUI `view` command requires an interactive terminal (TTY)
- Homebrew/winget channels are deferred; GitHub Release + crates.io are primary

### Upgrade Notes
- Fresh install recommended for first v0.1 use
- Re-run trust challenge after upgrade:
```bash
scripts/demo_quickcheck.sh /tmp/panopticon_demo_run
```
```

## 2) X Thread Draft

Post 1:

`Panopticon Suite v0.1.0 is out: deterministic, local-first run evidence for AI agent workflows. It records truth as append-only EventLog and proves behavior under stress with replayable artifacts.`

Post 2:

`Proof surfaces are built-in, not hand-wavy: metrics.json, viewmodel.hash, ansi.capture, timetravel.capture. Same fixture + same invariants => same viewmodel hash.`

Post 3:

`Share-safe export is refusal-first. If secrets are detected, export fails with a concrete refusal report showing where and why.`

Post 4:

`Trust challenge takes ~1 command path:
scripts/demo_quickcheck.sh /tmp/panopticon_demo_run
Look for tier_a_drops=0 and stable hash evidence.`

Post 5:

`Repo + release notes include exact verification and rollback flow for release artifacts and provenance checks.`

## 3) LinkedIn Post Draft

`Panopticon Suite v0.1.0 is now available.

This release focuses on deterministic run evidence for AI agent workflows:
- append-only EventLog as canonical truth
- stress Tour proof artifacts for reproducibility
- share-safe export posture with explicit refusal reporting

What I care about most in this release is trust through verification, not marketing claims. The repo includes command-level verification logs and release artifact verification flow.

If you want a quick technical pass, run:

scripts/demo_quickcheck.sh /tmp/panopticon_demo_run

and inspect tier_a_drops + hash outputs directly.`

## 4) Short Article Outline (technical)

Title:

`Deterministic Agent Run Evidence: Building Panopticon v0.1`

Outline:

1. Problem framing
- Most agent tooling has logs, but weak replay guarantees under load.
- Need: canonical ordering + reproducible projection + safe sharing.

2. Design choices
- EventLog truth model with append-writer `commit_index` assignment.
- Reducer/projection split and deterministic hash surfaces.
- Backpressure model where truth stays intact and projection degrades.

3. Proof over promises
- Tour artifact contracts.
- README verification workflow.
- Release artifact checksums + provenance attestation.

4. Failure modes and refusal-first export
- Secret scanner behavior.
- Refusal report path and operator action model.

5. What is deferred
- Homebrew/winget channels.
- Post-v0.1 UX polish rounds.

6. Closing
- How to run trust challenge in under 2 minutes.
- Where to file findings.

## 5) Asset Mapping (for posts)

Use canonical assets only:

- Incident view: `docs/assets/readme/incident-lens.txt`
- Forensic view: `docs/assets/readme/forensic-lens.txt`
- Degraded truth HUD: `docs/assets/readme/truth-hud-degraded.txt`
- Refusal output: `docs/assets/readme/export-refusal.txt`
- Artifact summary: `docs/assets/readme/artifacts-view.txt`

## 6) Messaging Guardrails

Do not claim:

- hosted SaaS deployment
- universal package manager support in v0.1
- stronger guarantees than verified by `docs/README_VERIFICATION.md`

Always include:

- one concrete command users can run now
- one concrete trust signal (`tier_a_drops`, hash stability, or checksum verification)
