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
- Runs `panopticon tour --stress` and checks `ansi.capture` includes:
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

`bakeoff-report.json` uses schema `panopticon-competitor-bakeoff-v1` for easy reuse in launch posts and benchmark narratives.

## Notes

- The harness is intentionally objective and reproducible; it does not rely on subjective visual claims.
- Use the same fixture and command path when presenting comparisons.
