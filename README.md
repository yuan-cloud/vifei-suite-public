# Panopticon Suite

Panopticon is a deterministic, local-first flight recorder for AI agent runs.

It records canonical run evidence as an append-only EventLog and produces replayable proof artifacts that let you verify behavior under stress.

## Value Proposition

- Deterministic replay: same input run history yields the same projection hash.
- Truth-first under pressure: canonical truth stays intact while only projection quality degrades.
- Share-safe export posture: unsafe exports are refused with explicit refusal details.
- Local-first operation: no daemon required for v0.1.

## See It Working

Run a full stress tour on the bundled fixture:

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir tour-output
```

You should see output confirming artifact creation:

- `metrics.json`
- `viewmodel.hash`
- `ansi.capture`
- `timetravel.capture`

Inspect the generated artifacts:

```bash
ls -1 tour-output
cat tour-output/viewmodel.hash
cat tour-output/metrics.json
```

Reference captures and visuals for this README are in `docs/assets/readme/`:

- `docs/assets/readme/incident-lens.txt`
- `docs/assets/readme/forensic-lens.txt`
- `docs/assets/readme/truth-hud-degraded.txt`
- `docs/assets/readme/export-refusal.txt`
- `docs/assets/readme/architecture.mmd`

## Quickstart

### 1) Build and check CLI surface

```bash
cargo run -p panopticon-tui --bin panopticon -- --help
```

### 2) Run deterministic stress tour

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir tour-output
```

### 3) Verify determinism quickly

```bash
cat tour-output/viewmodel.hash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir tour-output-rerun
cat tour-output-rerun/viewmodel.hash
```

Expected result: both hash files match.

### 4) View an EventLog

```bash
cargo run -p panopticon-tui --bin panopticon -- view docs/assets/readme/sample-eventlog.jsonl
```

Run this in a real interactive terminal (TTY). Non-interactive runners will fail to initialize terminal mode.

### 5) Export an EventLog with share-safe checks

```bash
cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output out/bundle.tar.zst --refusal-report out/refusal-report.json
```

## Why Trust This

Trust claims are testable in this repo.

- Canonical replay order is `commit_index`, assigned by one append writer.
- Deterministic reducer + projection paths are exercised in the test suite.
- Tour emits machine-checkable proof artifacts.
- Docs guard prevents constitutional drift into non-constitutional docs.

Key governance docs:

- `docs/CAPACITY_ENVELOPE.md`
- `docs/BACKPRESSURE_POLICY.md`

## Trust Challenge

You can challenge core claims directly:

1. Run `cargo test`.
2. Run the same Tour command twice on `fixtures/large-stress.jsonl`.
3. Compare `viewmodel.hash` outputs.
4. Confirm `tier_a_drops` in `metrics.json` is `0`.

If these checks fail, treat claims as unproven and investigate before release.

## Architecture

Panopticon separates canonical truth from derived views:

```text
Agent Cassette JSONL
        |
        v
Importer -> Append Writer (assigns commit_index)
        |
        v
Append-only EventLog + Blob Store  <-- canonical truth
        |
        v
Reducer (pure) -> Projection (deterministic)
        |
        v
ViewModel -> TUI lenses + Truth HUD
        |
        v
Tour artifacts (metrics.json, viewmodel.hash, ansi.capture, timetravel.capture)
```

Workspace crates:

- `crates/panopticon-core`: event schema, append writer, reducer, projection.
- `crates/panopticon-import`: Agent Cassette importer.
- `crates/panopticon-export`: bundle export and share-safe scanning.
- `crates/panopticon-tour`: stress harness and proof artifact emission.
- `crates/panopticon-tui`: CLI and terminal UI lenses.

## Proof Artifacts

`panopticon tour --stress` emits these artifacts:

- `metrics.json`: stress-run counters and policy outcomes.
- `viewmodel.hash`: deterministic projection hash (BLAKE3 hex).
- `ansi.capture`: deterministic ANSI summary capture.
- `timetravel.capture`: seek/replay capture data.

These shapes are part of v0.1 contract and validated in tests.

## Status

Panopticon v0.1 is in active implementation.

- Core determinism, Tour artifacts, and share-safe export flows are implemented.
- Release docs, launch assets, and final trust-verification polish are being completed through tracked beads.

Track execution in `PLANS.md`, `.beads/issues.jsonl`, and `docs/README_LAUNCH_PLAN.md`.

## Contributing

Development loop:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Process rules and quality gates are defined in `AGENTS.md`.

## Troubleshooting

### `--stress flag is required`

`tour` intentionally refuses non-stress mode in v0.1. Add `--stress`.

### `Export without secret scanning is not supported`

`export` requires `--share-safe`.

### Tour hash mismatch across reruns

Treat as determinism regression. Re-run on an idle machine, then inspect recent changes in reducer/projection paths.

### TUI view fails on fixture path

`view` expects EventLog JSONL, not Agent Cassette fixture JSONL. Use an EventLog path.
