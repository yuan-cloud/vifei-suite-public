# README Launch Plan (Post-Stabilization)

This plan defines how Panopticon's GitHub presentation will be made launch-ready without drifting from core engineering priorities.

Status: planned, not active implementation.

## Goal

Make the repository instantly understandable and trustworthy for new users, hiring reviewers, and contributors by optimizing for:

1. See it (high-quality screenshots/visuals)
2. Run it (copy-paste quickstart)
3. Verify it (determinism and safety checks)

## Why This Matters

- For this project, docs quality is part of product correctness signaling, not marketing polish.
- Panopticon claims deterministic behavior and truthful degradation; README must show executable proof.
- High-signal docs increase adoption and strongly improve senior-level evaluation of architecture and judgment.

## Activation Timing

Do not run this track while core behavior is still changing quickly.

Activate this plan only after:

1. Current core open beads are complete and reviewed.
2. One optimization/refinement round is complete (so screenshots and wording do not churn).
3. Main branch is stable enough for docs freeze pass.

## Deliverables

1. Top-level README rewrite with proof-first structure.
2. Curated screenshots and one architecture diagram in `docs/assets/readme/`.
3. Reproducible "Trust Challenge" command sequence (determinism + export refusal).
4. Docs verification pass confirming every command and claim.

## README Structure Contract

The README should contain these sections in this order:

1. Title + 2-sentence value proposition.
2. "See It Working" (screenshots/GIFs).
3. "60-Second Quickstart" (exact commands).
4. "Why Trust This" (I1..I5 in plain language, link to docs).
5. "Verify In 3 Steps (Trust Challenge)".
6. Architecture overview diagram (truth vs projection).
7. Proof artifacts (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`).
8. Current status + non-goals + roadmap.
9. Contributing + troubleshooting.

## Quality Bar (Must Pass)

1. Every command block is copy-paste tested.
2. Every claim has a reproducible check or source link.
3. Screenshots reflect current UI behavior and lens states.
4. Markdown is readable/scannable (short sections, descriptive headings).
5. No constitution duplication from guarded snippets.
6. Dark/light readability considered for images.

## Coordination Model

Roles:

- Implementer: writes README and captures assets.
- Reviewer: independent docs QA (findings-first, PASS/FAIL).
- Coordinator: enforces scope, timing, and final quality gate.

Workflow:

1. Claim bead and confirm ID.
2. Reserve narrow files (`README.md`, `docs/assets/readme/*`, `docs/README_LAUNCH_PLAN.md` as needed).
3. Implement minimal focused diff.
4. Run required checks (`cargo test` minimum for docs-only changes).
5. Send handoff with SHA + verification notes.
6. Independent review (PASS/FAIL).
7. Fix-forward if needed.

## Suggested Bead Sequence

1. README-PLAN: finalize launch plan and acceptance checklist.
2. README-CORE: rewrite README text structure and trust narrative.
3. README-ASSETS: produce screenshots/diagram assets.
4. README-VERIFY: validate commands and trust challenge end-to-end.
5. README-REVIEW: independent polish/reliability review and final edits.

Dependencies should ensure this sequence runs only after stabilization milestones are complete.

## Reference Standards

- GitHub docs writing best practices.
- Diataxis documentation framework.
- Rust API guidelines (clarity, correctness).
- OpenSSF/NIST secure software lifecycle guidance for verifiable claims.

These references guide style and rigor; repository constitution docs remain normative for Panopticon behavior.
