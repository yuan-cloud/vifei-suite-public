# Panopticon Showcase

Presentation-first preview for the project’s premium visual profile.

## One-command visual demo

```bash
cargo run -p panopticon-tui --bin panopticon -- \
  view docs/assets/readme/sample-eventlog.jsonl --profile showcase
```

## Why this matters

Panopticon keeps truth deterministic while allowing a higher-end presentation layer for demos and public-facing storytelling.

- Truth path is unchanged.
- `commit_index` ordering remains canonical.
- `viewmodel.hash` verification flow remains the same.

## Visual gallery

Incident Lens (showcase):

![Incident Lens Showcase](../assets/readme/incident-lens-showcase.svg)

Forensic Lens (showcase):

![Forensic Lens Showcase](../assets/readme/forensic-lens-showcase.svg)

Truth HUD (showcase):

![Truth HUD Showcase](../assets/readme/truth-hud-showcase.svg)

## Demo track

```bash
scripts/demo/determinism_duel.sh --fast
scripts/demo/refusal_radar.sh --fast
scripts/demo/live_incident_wall.sh --fast
```

Use `--full` for stress-grade runs:

```bash
scripts/demo/determinism_duel.sh --full
scripts/demo/refusal_radar.sh --full
scripts/demo/live_incident_wall.sh --full
```

Expected signals:

1. Determinism Duel prints identical hash A/hash B and `PASS`.
2. Refusal Radar prints blocked field details and refusal `PASS`.
3. Live Incident Wall prints showcase asset paths and `PASS`.
