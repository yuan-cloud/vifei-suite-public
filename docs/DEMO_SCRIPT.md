# Demo Script (v0.1)

This is the reproducible demo flow for launch media (`bd-3qq.1`).

## Demo Goal (60-90 seconds)

Show three proof moments, in order:

1. Deterministic stress tour produces stable proof artifacts.
2. Trust challenge check: Tier A drops are zero and hash is stable.
3. Share-safe export succeeds on clean input and refuses unsafe input.

## Preflight

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## One-command demo quickcheck

```bash
scripts/demo_quickcheck.sh /tmp/panopticon_demo_run
```

Outputs:

- `/tmp/panopticon_demo_run/tour/metrics.json`
- `/tmp/panopticon_demo_run/tour/viewmodel.hash`
- `/tmp/panopticon_demo_run/export-success.txt`
- `/tmp/panopticon_demo_run/export-refused.txt`

## Presenter Script (spoken beats)

1. "Panopticon records deterministic run evidence and stays truthful under stress."
2. Run:

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/panopticon_demo_run/tour
```

3. "Now check proof outputs and trust posture."

```bash
cat /tmp/panopticon_demo_run/tour/viewmodel.hash
python3 - <<'PY'
import json
m=json.load(open('/tmp/panopticon_demo_run/tour/metrics.json'))
print('tier_a_drops=', m['tier_a_drops'])
print('degradation_level_final=', m['degradation_level_final'])
PY
```

4. "Clean export succeeds with share-safe checks."

```bash
cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output /tmp/panopticon_demo_run/export/bundle.tar.zst --refusal-report /tmp/panopticon_demo_run/export/refusal-report.json
```

5. "Unsafe export is refused with concrete findings."

```bash
cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-refusal-eventlog.jsonl --share-safe --output /tmp/panopticon_demo_run/export/refusal-bundle.tar.zst --refusal-report /tmp/panopticon_demo_run/export/refusal-report-refused.json || true
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
asciinema rec -i 1 /tmp/panopticon_demo_run/demo.cast
```

Then run the presenter script commands and stop recording with `Ctrl-D`.

If `vhs` is preferred, use a tape that executes `scripts/demo_quickcheck.sh` and trims pauses.

## Failure Handling

- If `tier_a_drops != 0`, stop recording and open a blocker bead.
- If clean export refuses, regenerate samples with `cargo run -p panopticon-tui --bin capture_readme_assets` and re-run quickcheck.
- If refusal export succeeds, stop; secret scanner expectations are broken.
