# Immediate Future Showcase Checklist

This checklist tracks remaining visible upgrades after the current showcase baseline.

## Shipped baseline (already done)

1. Showcase profile visuals are implemented in TUI (`standard` and `showcase`).
2. Deterministic showcase SVG/TXT gallery is generated under `docs/assets/readme/`.
3. Cast-first recording lane is available via `scripts/capture_showcase_cast.sh` (`asciinema`).
4. Showcase page exists at `docs/showcase/index.md` with runnable demo commands.
5. Trust-first short demo cut exists at `scripts/demo/trust_demo_cut.sh` with contract check `scripts/testing/check_trust_demo_cut_contract.sh`.
6. Visual showcase short cut exists at `scripts/demo/visual_showcase_cut.sh` with contract check `scripts/testing/check_visual_showcase_cut_contract.sh`.

## Priority 1 (next)

1. Launch media bundle
- Consolidate trust and visual cuts into final launch bundle runbook for Product Hunt/social.

2. Desktop/mobile showcase layout refinement
- Separate desktop scan path from narrow/mobile scan path in showcase docs.
- Keep command blocks short enough for narrow widths while preserving exact copy/paste behavior.
- Add explicit proof card ordering: trust signal, deterministic proof, refusal proof, incident wall.

3. Adapter-facing demo tracks
- Provide per-target demo snippets (CLI human mode, robot mode, export refusal mode).
- Show how each mode maps to real user outcomes.

## Priority 2 (immediately after)

1. Web renderer exploration branch
- Evaluate lightweight interactive view driven by generated artifacts.
- Keep this separate from truth-path runtime crates.

2. Publish-ready cast packaging
- Add a short publish guide for `.cast` upload/embed paths.
- Keep canonical evidence artifacts local and deterministic; casts are presentation assets.

## Constraints

1. No truth-path semantic changes in showcase tasks.
2. Deterministic assets remain source of visual truth.
3. Any non-deterministic media tooling is optional and non-canonical.
