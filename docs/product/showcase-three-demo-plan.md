# Showcase Three-Demo Plan

Objective: ship three high-impact demos that make Panopticon visually compelling while reinforcing deterministic trust and share-safe safety posture.

## Demo A: Determinism Duel

Narrative:
Two independent replays of the same stress fixture converge on the same `viewmodel.hash`.

What users see:
1. Split-lane replay summary.
2. Matching final proof hash.
3. Clear pass/fail badge.

Implementation scope:
1. Add demo script wrapper for dual-run replay and hash comparison.
2. Generate a dedicated capture asset set for this flow.
3. Add one short launch clip script path (optional tooling).

Acceptance:
1. Script exits non-zero on hash mismatch.
2. README/showcase includes copy-paste command and expected output.

## Demo B: Refusal Radar

Narrative:
Export refuses unsafe bundles and reports blocked fields with explicit reasons.

What users see:
1. Refusal summary panel.
2. Blocked field paths and patterns highlighted.
3. Direct path to `refusal-report.json`.

Implementation scope:
1. Add presentation capture for refusal-report walkthrough.
2. Add showcase section for refusal semantics.
3. Keep all examples tied to deterministic fixture artifacts.

Acceptance:
1. Refusal demo shows at least one blocked secret pattern with redacted match.
2. Artifact links resolve from docs and README.

## Demo C: Live Incident Wall

Narrative:
High-signal incident/forensic view that remains honest under pressure.

What users see:
1. Incident lens triage summary.
2. Forensic lens event drilldown.
3. Truth HUD pressure/degradation confession always visible.

Implementation scope:
1. Extend showcase capture set with “wall” framing and captions.
2. Add narrow-width variant to show responsive behavior.
3. Add one deterministic “tour-driven wall” visual page section.

Acceptance:
1. Showcase assets include incident, forensic, and Truth HUD wall views.
2. Command-to-asset traceability is documented.

## Dependency and sequencing

1. Sequence 1: Determinism Duel (proof-first anchor).
2. Sequence 2: Refusal Radar (safety-first anchor).
3. Sequence 3: Live Incident Wall (premium visual anchor).

Dependencies:
1. Existing showcase profile and SVG capture pipeline.
2. Existing stress fixture and tour artifact generation.
3. README/showcase page docs flow.

## Borrowing strategy from FrankenTUI-style momentum

Apply:
1. Demo-first storytelling.
2. Premium terminal presentation language.
3. Fast “try now” path with immediate visual payoff.

Do not compromise:
1. Truth-path invariants.
2. Canonical ordering semantics.
3. Reproducible proof claims.

## Deliverables checklist

1. Demo script set under `scripts/demo/`.
2. Captured assets under `docs/assets/readme/` and/or `docs/assets/showcase/`.
3. README + showcase page sections per demo.
4. One launch-ready command block per demo.

## Current implementation status

1. `scripts/demo/determinism_duel.sh` implemented with `--fast`/`--full`.
2. `scripts/demo/refusal_radar.sh` implemented with `--fast`/`--full`.
3. `scripts/demo/live_incident_wall.sh` implemented with `--fast`/`--full`.
4. `scripts/testing/demo_smoke.sh` added for CI drift protection.
