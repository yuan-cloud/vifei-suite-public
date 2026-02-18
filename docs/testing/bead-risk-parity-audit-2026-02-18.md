# Bead Closure vs Risk Register Parity Audit

Generated at: `2026-02-18T00:00:00Z`

## Taxonomy

- `covered_exact`: exact `## <bead-id>` risk heading exists.
- `covered_milestone_alias`: milestone heading (for example `M4`, `M5.1`) covers legacy bead naming.
- `covered_parent_rollup`: parent/feature bead can rely on fully-covered child evidence.
- `exempt_program_meta`: program/track orchestration bead exempt from per-bead risk entry.
- `gap_requires_entry`: no accepted evidence mapping; requires remediation.

## Summary

- Total closed beads audited: `185`
- Action-required gaps: `22`

Classification counts:
- `covered_exact`: `122`
- `covered_milestone_alias`: `33`
- `covered_parent_rollup`: `8`
- `gap_requires_entry`: `22`

## Remediation Plan

1. Add CI guard for non-exempt closed beads (`A2`).
2. Backfill historical `gap_requires_entry` beads or apply explicit exemption rationale (`A3`).
3. Keep parent/feature rollups auditable by preserving child evidence links.

## Initial Action Queue (gap IDs)

- `bd-1wh`
- `bd-24k.1`
- `bd-24k.2`
- `bd-24k.3`
- `bd-2b4`
- `bd-3fw.1`
- `bd-3fw.2`
- `bd-3fw.3`
- `bd-80v`
- `bd-bjv`
- `bd-bjv.1`
- `bd-bjv.5`
- `bd-bjv.7`
- `bd-bjv.8`
- `bd-c7m`
- `bd-c7m.1`
- `bd-d7c`
- `bd-d7c.1`
- `bd-d7c.2`
- `bd-d7c.3`
- `bd-lh8`
- `bd-yh1`
