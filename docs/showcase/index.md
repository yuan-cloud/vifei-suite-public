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

## Proof track (determinism)

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir out-a
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir out-b
cat out-a/viewmodel.hash
cat out-b/viewmodel.hash
```

Expected result: both hashes match.
