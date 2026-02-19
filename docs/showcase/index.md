# Vifei Showcase

Presentation-first preview for the project's premium visual profile.

## One-command visual demo

```bash
cargo run -p vifei-tui --bin vifei -- \
  view docs/assets/readme/sample-eventlog.jsonl --profile showcase
```

## What this demonstrates

Vifei keeps truth deterministic while supporting a higher-end presentation layer for demos and public storytelling.

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

### Adapter-facing human CLI track

Operator-readable command path:

`docs/showcase/adapter-human-cli-track.md`

### Adapter-facing robot JSON track

Automation-readable command path:

`docs/showcase/adapter-robot-json-track.md`

### Adapter-facing refusal and safety track

Security and governance command path:

`docs/showcase/adapter-refusal-safety-track.md`

### Visual launch cut (45-90s)

```bash
scripts/demo/visual_showcase_cut.sh /tmp/vifei_visual_cut
```

This produces a concise visual proof summary with deterministic hashes for showcase and narrow assets.

### Desktop flow (operator presentation)

```bash
scripts/demo/determinism_duel.sh --fast
scripts/demo/refusal_radar.sh --fast
scripts/demo/live_incident_wall.sh --fast
scripts/demo/competitor_bakeoff.sh --fast
```

Use `--full` for stress-grade runs:

```bash
scripts/demo/determinism_duel.sh --full
scripts/demo/refusal_radar.sh --full
scripts/demo/live_incident_wall.sh --full
scripts/demo/competitor_bakeoff.sh --full
```

### Mobile or narrow-width flow (proof-first)

```bash
scripts/demo/determinism_duel.sh --fast
scripts/demo/refusal_radar.sh --fast
cat .tmp/competitor-bakeoff/run-*/bakeoff-report.json
```

Expected signals:

1. Determinism Duel prints identical hash A/hash B and `PASS`.
2. Refusal Radar prints blocked field details and refusal `PASS`.
3. Live Incident Wall prints showcase asset paths and `PASS`.
4. Competitor Bakeoff writes `bakeoff-report.json` with deterministic/stability/refusal/explainability checks.

## Verification bundle

```bash
mkdir -p .tmp/final-audit
scripts/testing/check_bead_closure_evidence.py \
  --audit-output-json .tmp/final-audit/bead-risk-parity.json \
  --audit-output-markdown .tmp/final-audit/bead-risk-parity.md
scripts/testing/validate_defer_register.py docs/testing/defer-register-v0.1.json
scripts/testing/check_coverage_contract.sh
scripts/testing/demo_smoke.sh .tmp/final-audit/demo-smoke
```
