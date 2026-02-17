# Panopticon v0.1 Launch + Distribution Plan (Post-Stabilization)

Status: planning artifact. Do not execute this track while core milestone beads are still unstable.

## Scope

This plan answers four questions for v0.1:

1. How Panopticon should be shipped (distribution, not cloud deployment)
2. What launch is likely to look like
3. How to add personality/"wow" moments without breaking trust posture
4. How to sequence implementation and comms work with bead-driven execution

## Critical Insight Check (Current Repo State)

- Root `README.md` is currently missing, so "launch-quality docs" starts with creating a top-level README.
- Existing launch track (`bd-x7q.*`) already covers README and assets, but not full release-channel operations.
- Core M6/M7 work has been active; launch execution should remain gated on stabilization and review completion.

## Do We Need "Deployment" for v0.1?

For v0.1, Panopticon does not need a hosted runtime deployment.

Recommended interpretation:

- **No SaaS deployment yet**
- **Yes to distribution deployment**:
  - GitHub Releases (signed checksums + binaries)
  - crates.io publish for applicable crates
  - optional package manager channels (Homebrew, winget)

Rationale: Panopticon is local-first and deterministic. Trust is built by reproducible local execution and artifact verification, not by uptime metrics.

## Release Surfaces (Priority Order)

1. **GitHub Release (must-have)**
- Linux/macOS/Windows artifacts
- SHA256/BLAKE3 checksums
- release notes: what changed, what was verified, known limits

2. **crates.io publish (must-have for Rust audience)**
- publish only stable crates
- run `cargo publish --dry-run` before final publish
- include docs.rs-friendly crate docs and minimal examples

3. **Homebrew tap (high-value convenience)**
- one-line install path for macOS/Linux developers
- formula update automation can come after first manual release

4. **winget (optional but strong reach)**
- useful if Windows adoption becomes visible after first launch

5. **Container image (optional / low priority)**
- only if users request a disposable sandbox for quick trials
- not a substitute for native binary distribution

## Product-Market Positioning (for Launch Messaging)

Panopticon should position as:

- deterministic local-first run evidence cockpit
- replayable truth model under overload (truth preserved, projection degrades)
- share-safe export with refusal reports

Avoid positioning as:

- generic chat UI shell
- cloud observability replacement for hosted LLM platforms

## Competitor/Comparable Landscape (2026 snapshot)

These are adjacent, not identical:

1. **Langfuse**
- strategy: open-source observability + managed cloud, strong docs, broad integrations
- lesson: invest in integration docs and eval workflow examples

2. **Arize Phoenix**
- strategy: open-source tracing/eval with notebook-first and framework-first workflows
- lesson: make "time-to-first-insight" very short

3. **Helicone**
- strategy: API gateway + observability controls with hosted/on-prem story
- lesson: users value clear control points and clear pricing/usage framing

4. **Comet Opik**
- strategy: OSS LLM eval/observability with explicit enterprise option
- lesson: side-by-side OSS + practical production paths reduce adoption friction

Panopticon differentiation to emphasize:

- canonical local event truth + deterministic replay hash surfaces
- explicit overload honesty model
- evidence-bundle workflow for audit/share

## FrankenTUI Reference Pattern (What to Borrow)

FrankenTUI's visible go-to-market choices worth copying:

1. Demo-first onboarding (`cargo run ...` immediately)
2. Strong README narrative with architecture explanation
3. Public releases + crate publishing progression
4. Visual proof artifacts (screenshots/showcase demos)

Adaptation for Panopticon:

- keep a demo/tour command as first-run path
- show proof artifacts (`metrics.json`, `viewmodel.hash`, capture files) early in README
- keep mathematical/technical depth in docs, but prioritize runnable commands at top

## "Pretty / Personality / Viral" Without Breaking Trust

Allowed and recommended:

- a dedicated "demo profile" for aesthetically rich terminal moments
- curated capture scenarios for sharing clips/images
- consistent visual identity for screenshots and release assets

Not allowed in core truth path:

- randomness in deterministic proof paths
- aesthetics that alter canonical truth semantics
- hidden behavior that weakens auditability

Rule of thumb: personality belongs in presentation mode and docs assets, not in deterministic core invariants.

## Launch Outcome Forecast (Realistic)

Likely phase-1 outcome:

- strong traction in Rust/CLI/TUI/agent-tooling communities
- low-friction adoption by technical users if install + demo is under 2 minutes
- moderate social spread unless clips and narrative are tightly packaged

Higher-variance "viral" upside requires:

- one unmistakable demo sequence people want to repost
- clear "why this is different" framing in first 15 seconds
- copy-paste install that always works

## Content Formats to Ship Together

1. GitHub README (primary canonical entry)
2. GitHub Release notes (technical changelog + verification checklist)
3. Short launch post (blog/Medium/dev.to)
4. X/LinkedIn thread with terminal clip and concrete claims
5. 60-90 second walkthrough video (or asciinema embed)

## Timing: When to Research vs Execute

1. **Now**: finalize release/distribution plan and launch message architecture
2. **After stabilization gate passes**: produce README, assets, and reproducibility checks
3. **Release week**: execute publish/distribute + social assets
4. **Post-release week**: synthesize feedback into next bead batch

## Execution Workflow (Agent-Safe)

1. Planning pass
- finalize this plan doc
- create beads and dependencies

2. Stabilization gate
- ensure open core beads are complete and independently reviewed

3. Packaging gate
- release artifacts pipeline implemented and tested

4. Docs gate
- README + screenshots + trust challenge verified line-by-line

5. Launch gate
- release publish + communication bundle shipped in one coordinated window

6. Post-launch learning gate
- capture feedback and convert into prioritized beads

## README Launch Sequencing Constraints (`bd-x7q.*`)

These constraints are mandatory for launch-docs execution in this repo.

1. `bd-x7q.1` must complete before any README writing:
- finalize this plan as the single coordination source
- lock acceptance criteria for `bd-x7q.2` to `bd-x7q.5`

2. `bd-x7q.2` (`README.md` core rewrite) must complete before asset capture:
- assets are captured against final section structure, not drafts

3. `bd-x7q.3` (assets) must complete before verification:
- verification references concrete screenshot/diagram paths

4. `bd-x7q.4` (verification) must complete before independent QA:
- QA validates what was actually executed and recorded

5. `bd-x7q.5` is a findings-first gate:
- unresolved factual or reproducibility findings block launch tagging

## Acceptance Checklist for `bd-x7q.1`

`bd-x7q.1` is done only when all items below are true:

- `docs/README_LAUNCH_PLAN.md` explicitly defines ordered `bd-x7q.1` → `bd-x7q.5` constraints.
- The plan states which bead owns each artifact type:
  - README structure/content: `bd-x7q.2`
  - screenshots/diagram assets: `bd-x7q.3`
  - command/trust-step validation: `bd-x7q.4`
  - independent QA/polish: `bd-x7q.5`
- The plan includes a hard gate that unresolved verification findings block release tagging.
- The plan avoids constitutional duplication (no copied threshold/ladder tables from constitution docs).
- The bead and risk register are updated in the same commit.

## Proposed Bead Families (new)

- `RELEASE-OPS`: binary packaging, checksums, release checklist, publish flow
- `LAUNCH-MESSAGE`: README narrative, positioning, comparison framing
- `LAUNCH-MEDIA`: screenshots/clips/demo script
- `LAUNCH-DISTRIBUTION`: GitHub release + crates publish + package-manager follow-through
- `POST-LAUNCH-LEARN`: issue triage rubric and feedback ingestion loop

## Release Ops Artifacts

- Packaging matrix and go/no-go checklist: `docs/RELEASE_PACKAGING_CHECKLIST.md`
- Trust verification and attestation rollback: `docs/RELEASE_TRUST_VERIFICATION.md`

## Sources

- Cargo publishing reference: https://doc.rust-lang.org/cargo/reference/publishing.html
- Cargo publish command: https://doc.rust-lang.org/cargo/commands/cargo-publish.html
- crates.io updates (trusted publishing context):
  - https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/
  - https://blog.rust-lang.org/2026/01/21/crates-io-development-update/
- GitHub Releases docs: https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
- GitHub Pages publish source docs: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
- Homebrew tap guide: https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap
- winget packaging docs: https://learn.microsoft.com/en-us/windows/package-manager/package/
- cargo-dist docs: https://opensource.axo.dev/cargo-dist/
- ASTRAL-inspired release automation pattern (cargo-dist, release examples): https://github.com/astral-sh/uv
- Langfuse docs: https://langfuse.com/docs
- Arize Phoenix docs: https://arize.com/docs/phoenix
- Helicone docs: https://docs.helicone.ai/
- Opik docs: https://www.comet.com/docs/opik/
- FrankenTUI site and repo:
  - https://frankentui.com/
  - https://github.com/Dicklesworthstone/frankentui
- Terminal capture tooling:
  - https://asciinema.org/
  - https://github.com/charmbracelet/vhs
