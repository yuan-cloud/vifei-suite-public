# Bead Closure Evidence Policy (v0.1)

Purpose: define auditable rules that determine when a closed bead is considered evidence-complete against `docs/RISK_REGISTER.md`.

Canonical audit command:

```bash
scripts/testing/audit_bead_risk_parity.py \
  --output-json docs/testing/bead-risk-parity-audit-2026-02-18.json \
  --output-markdown docs/testing/bead-risk-parity-audit-2026-02-18.md
```

## Classification rules

- `covered_exact`
  - Definition: `docs/RISK_REGISTER.md` contains an exact heading `## <bead-id> ...`.
  - Policy: accepted as complete.

- `covered_milestone_alias`
  - Definition: legacy milestone heading (for example `M4`, `M5.1`) maps to a closed bead title that carries the same milestone token.
  - Policy: accepted for historical compatibility.
  - Forward policy: new work should prefer exact bead-id headings.

- `covered_parent_rollup`
  - Definition: a feature/program bead has parent-child structure and all child beads are covered.
  - Policy: accepted for rollup beads.

- `exempt_program_meta`
  - Definition: orchestration-only program/track beads (`PROGRAM:*`, `TRACK-*`).
  - Policy: exempt from per-bead risk entries.

- `gap_requires_entry`
  - Definition: no accepted evidence mapping exists.
  - Policy: must be resolved before closure parity is green.

## Remediation protocol

1. Generate parity audit outputs.
2. Resolve all `gap_requires_entry` rows by:
   - adding exact risk entry, or
   - adding explicit exemption rationale in policy/tracker context.
3. Re-run audit and confirm no unresolved critical gaps for targeted closure scope.
4. Enforce in CI via closure-evidence guard (follow-up bead).

## CI enforcement guard

Guard command (fails on unresolved non-exempt gaps):

```bash
scripts/testing/check_bead_closure_evidence.py \
  --audit-output-json .tmp/bead-closure-evidence/bead-risk-parity-audit.json \
  --audit-output-markdown .tmp/bead-closure-evidence/bead-risk-parity-audit.md
```

Guard behavior:

- Re-runs the parity audit deterministically.
- Loads explicit temporary exemptions from:
  - `docs/testing/bead-closure-evidence-exemptions-v0.1.json`
- Fails with `CONTRACT_FAIL[CLOSE-001]` when any `gap_requires_entry` bead is not listed in that exemption ledger.
- Emits replay and fix guidance in output.

Policy constraints:

- Exemptions must be explicit, ID-based, and include rationale.
- Exemption ledger is temporary and must shrink as `bd-25a0` backfills historical gaps.
- New closure gaps must never be silently tolerated.

## Notes

- This policy governs process evidence only. It does not replace code/test quality gates.
- The two constitutional docs remain authoritative for runtime semantics:
  - `docs/CAPACITY_ENVELOPE.md`
  - `docs/BACKPRESSURE_POLICY.md`
