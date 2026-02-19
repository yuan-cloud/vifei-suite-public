# Competitor Bakeoff Harness

Purpose: provide one deterministic command that produces objective proof artifacts for public comparison demos.

Command:

```bash
scripts/demo/competitor_bakeoff.sh --fast
```

Full run:

```bash
scripts/demo/competitor_bakeoff.sh --full
```

## What it proves

1. Determinism stability
- Runs dual replay via `scripts/demo/determinism_duel.sh`.
- Requires matching `viewmodel.hash` values.

2. Refusal semantics
- Runs share-safe refusal flow via `scripts/demo/refusal_radar.sh`.
- Requires refusal report with blocked items.

3. Explainability surface
- Runs `vifei tour --stress` and checks `ansi.capture` includes:
  - `Level:`
  - `Agg:`
  - `Pressure:`
  - `Drops:`
  - `Export:`
  - `Version:`

4. Incident evidence artifacts
- Runs one-command incident pack over deterministic sample input.
- Requires:
  - `manifest.json`
  - `compare/delta.json`
  - `replay/left.replay.json`
  - `replay/right.replay.json`

## Outputs

Each run writes a timestamped directory under `.tmp/competitor-bakeoff/` by default:

- `duel/`
- `radar/`
- `tour/`
- `incident-pack/`
- `bakeoff-report.json`

`bakeoff-report.json` uses schema `vifei-competitor-bakeoff-v1` for easy reuse in launch posts and benchmark narratives.

## CI-compatible evidence run

For an auditable final check pass that includes bakeoff artifacts:

```bash
mkdir -p .tmp/final-audit
scripts/testing/check_bead_closure_evidence.py \
  --audit-output-json .tmp/final-audit/bead-risk-parity.json \
  --audit-output-markdown .tmp/final-audit/bead-risk-parity.md
scripts/testing/validate_defer_register.py docs/testing/defer-register-v0.1.json
scripts/testing/check_coverage_contract.sh
scripts/testing/demo_smoke.sh .tmp/final-audit/demo-smoke
```

This produces:
- governance parity reports (`bead-risk-parity.json` + `.md`)
- fast demo evidence under `.tmp/final-audit/demo-smoke/`
- fast competitor bakeoff report at `.tmp/final-audit/demo-smoke/bakeoff/.../bakeoff-report.json`

## Notes

- The harness is intentionally objective and reproducible; it does not rely on subjective visual claims.
- Use the same fixture and command path when presenting comparisons.
