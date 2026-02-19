# Immediate Future Showcase Checklist

This checklist tracks the next visible upgrades after the current showcase profile + SVG gallery baseline.

## Priority 1 (next)

1. Scripted terminal video capture lane
- Use `scripts/capture_showcase_cast.sh` with `asciinema` when installed.
- Capture lane standard is cast-first (`.cast`) for replayable fidelity and fast iteration.
- Keep SVG/TXT outputs as canonical deterministic artifacts.

2. Showcase microsite
- Add `docs/showcase/index.md` using existing generated assets.
- Include one deterministic proof block (`viewmodel.hash` rerun match).
- Keep all commands copy/paste-ready and verified.

3. Launch media bundle
- Add one 45-90 second demo flow for Product Hunt/social.
- Keep one terminal-only “trust demo” and one visual “showcase demo.”

## Priority 2 (immediately after)

1. Web renderer exploration branch
- Evaluate lightweight interactive view driven by generated artifacts.
- Keep this separate from truth-path runtime crates.

2. Adapter-facing demo tracks
- Provide per-target demo snippets (CLI human mode, robot mode, export refusal mode).
- Show how each mode maps to real user outcomes.

## Constraints

1. No truth-path semantic changes in showcase tasks.
2. Deterministic assets remain source of visual truth.
3. Any non-deterministic media tooling is optional and non-canonical.
