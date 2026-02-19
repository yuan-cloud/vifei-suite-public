# Demo Script (v0.1)

This is the reproducible demo flow for launch media (`bd-3qq.1`).

## Demo Goal (60-90 seconds)

Show four proof moments, in order:

1. Deterministic stress tour produces stable proof artifacts.
2. Trust check: Tier A drops are zero and hash is stable.
3. Share-safe export succeeds on clean input and refuses unsafe input.
4. Competitor bakeoff report summarizes deterministic comparison signals.

Adapter-facing operator snippets:
- `docs/showcase/adapter-human-cli-track.md`
- `docs/showcase/adapter-robot-json-track.md`

## Preflight

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## One-command demo quickcheck

```bash
scripts/demo_quickcheck.sh /tmp/vifei_demo_run
```

## Trust demo cut (45-60s)

Run the trust-first short cut for launch clips:

```bash
scripts/demo/trust_demo_cut.sh /tmp/vifei_trust_cut fixtures/small-session.jsonl
```

It outputs `TRUST_DEMO_SUMMARY.txt` with:

- `deterministic_hash`
- `tier_a_drops=0`
- non-zero `blocked_items` from refusal-proof export

## Visual showcase cut (45-90s)

Run the visual cut for desktop and narrow proof assets:

```bash
scripts/demo/visual_showcase_cut.sh /tmp/vifei_visual_cut
```

It outputs `VISUAL_SHOWCASE_SUMMARY.txt` with deterministic asset hashes for:

- `incident-lens-showcase.svg`
- `forensic-lens-showcase.svg`
- `truth-hud-showcase.svg`
- `incident-lens-narrow-72.svg`

Outputs:

- `/tmp/vifei_demo_run/tour/metrics.json`
- `/tmp/vifei_demo_run/tour/viewmodel.hash`
- `/tmp/vifei_demo_run/export-success.txt`
- `/tmp/vifei_demo_run/export-refused.txt`
- `/tmp/vifei_demo_run/media-provenance.json`

Verify provenance quickly:

```bash
cargo run -p vifei-tour --bin media_provenance -- \
  --verify /tmp/vifei_demo_run/media-provenance.json \
  --base-dir /tmp/vifei_demo_run
```

Run media hygiene scan:

```bash
scripts/testing/check_media_hygiene.sh /tmp/vifei_demo_run
```

False-positive and override policy:

- Add narrow allowlist rules in `scripts/testing/media_hygiene_allowlist.txt` for known synthetic fixtures.
- Keep allowlist entries scoped to pattern + file context.
- Emergency override is explicit only: `VIFEI_HYGIENE_ALLOW_UNSAFE=1`.
- Do not use override in release publishing unless incident commander approval is recorded.

## Objective bakeoff check

```bash
scripts/demo/competitor_bakeoff.sh --fast /tmp/vifei_demo_run/bakeoff
cat /tmp/vifei_demo_run/bakeoff/run-*/bakeoff-report.json
```

## Presenter Script (spoken beats)

1. "Vifei records deterministic run evidence and stays truthful under stress."
2. Run:

```bash
cargo run -p vifei-tui --bin vifei -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/vifei_demo_run/tour
```

3. "Now check proof outputs and trust posture."

```bash
cat /tmp/vifei_demo_run/tour/viewmodel.hash
python3 - <<'PY'
import json
m=json.load(open('/tmp/vifei_demo_run/tour/metrics.json'))
print('tier_a_drops=', m['tier_a_drops'])
print('degradation_level_final=', m['degradation_level_final'])
PY
```

4. "Clean export succeeds with share-safe checks."

```bash
cargo run -p vifei-tui --bin vifei -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output /tmp/vifei_demo_run/export/bundle.tar.zst --refusal-report /tmp/vifei_demo_run/export/refusal-report.json
```

5. "Unsafe export is refused with concrete findings."

```bash
cargo run -p vifei-tui --bin vifei -- export docs/assets/readme/sample-refusal-eventlog.jsonl --share-safe --output /tmp/vifei_demo_run/export/refusal-bundle.tar.zst --refusal-report /tmp/vifei_demo_run/export/refusal-report-refused.json || true
```

6. "Incident comparison contract supports mixed formats."

```bash
cargo run -p vifei-tui --bin vifei -- compare \
  fixtures/small-session.jsonl \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --left-format cassette \
  --right-format eventlog
```

## Capture Asset List

Use these canonical static assets for README/social cards:

- `docs/assets/readme/incident-lens.txt`
- `docs/assets/readme/forensic-lens.txt`
- `docs/assets/readme/truth-hud-degraded.txt`
- `docs/assets/readme/export-refusal.txt`
- `docs/assets/readme/artifacts-view.txt`
- `docs/assets/readme/architecture.mmd`

## Optional terminal recording

If `asciinema` is installed:

```bash
scripts/capture_showcase_cast.sh --fast /tmp/vifei_demo_run/cast
```

This captures the deterministic demo flow with replayable terminal output.

## Launch bundle packaging

Create a release-friendly media bundle with replay notes and transcript:

```bash
scripts/demo/package_launch_bundle.sh .tmp/launch-media-bundle
```

Bundle outputs include:

- `trust-cut/TRUST_DEMO_SUMMARY.txt`
- `visual-cut/VISUAL_SHOWCASE_SUMMARY.txt`
- `COMMAND_ASSET_MAP.md`

## Failure Handling

- If `tier_a_drops != 0`, stop recording and open a blocker bead.
- If clean export refuses, regenerate samples with `cargo run -p vifei-tui --bin capture_readme_assets` and re-run quickcheck.
- If refusal export succeeds, stop; secret scanner expectations are broken.
