# A2 UX Improvement Proof · Narrow-Safe Incident Next Action (2026-02-17)

## Bead

- `bd-hov`
- UX lever: keep explicit next-step guidance visible in Incident Lens under narrow widths.

## Problem observed

From `docs/testing/ux-audit-a2-2026-02-17.md`:

- desktop incident view retained explicit next-step guidance,
- narrow incident flow had weaker/less reliable next-step guidance visibility during triage.

## Implementation

File:

- `crates/vifei-tui/src/incident_lens.rs`

Changes:

1. Added width-aware next-action message selection (`next_action_line`).
2. Added wrap-aware anomalies section height budgeting so the hint remains visible when it wraps.
3. Kept `Next action:` prefix stable across widths for modality contract consistency.
4. Added narrow regression test: `incident_lens_narrow_keeps_next_action_hint_visible`.

## Determinism + safety

- No reducer/projection/truth-path logic changed.
- No event ordering or hashing behavior changed.
- Change is confined to Incident Lens presentation copy/layout budgeting.

## Verification

Commands run:

```bash
cargo test -p vifei-tui --test modality_validation
cargo test -p vifei-tui incident_lens::tests::incident_lens_narrow_keeps_next_action_hint_visible
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Artifacts refreshed:

- `docs/assets/readme/incident-lens.txt`
- `docs/assets/readme/truth-hud-degraded.txt`

## Outcome

- Narrow triage now preserves explicit operator guidance with stable marker text.
- Desktop behavior remains semantically unchanged.
- Modality contract (`Next action:` marker across width buckets) remains green.
