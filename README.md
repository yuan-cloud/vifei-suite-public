# Panopticon Suite

[![CI](https://img.shields.io/github/actions/workflow/status/yuan-cloud/panopticon-suite/ci.yml?branch=main&label=CI)](https://github.com/yuan-cloud/panopticon-suite/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/tag/yuan-cloud/panopticon-suite?label=release)](https://github.com/yuan-cloud/panopticon-suite/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Deterministic, local-first run evidence for AI agent workflows.

Panopticon records canonical run truth as an append-only EventLog, then projects that truth into operator views and proof artifacts you can re-run and verify.

Presentation showcase: `docs/showcase/index.md`

## Why This Exists

Most agent workflows have logs but weak replay guarantees under stress. Panopticon keeps truth auditable when pressure rises.

- Canonical ordering uses `commit_index` from one append writer.
- Truth stays intact under overload; only projection quality degrades.
- Share-safe export refuses unsafe bundles and emits explicit refusal reports.

## 60-Second Quickstart

1. Run deterministic stress Tour:

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir tour-output
```

2. Confirm proof artifacts:

```bash
ls -1 tour-output
cat tour-output/viewmodel.hash
cat tour-output/metrics.json
```

3. Optional human-readable CLI surface check:

```bash
cargo run -p panopticon-tui --bin panopticon -- --human --help
```

Expected artifact files:

- `metrics.json`
- `viewmodel.hash`
- `ansi.capture`
- `timetravel.capture`

## Trust Signals (What You Can Verify Yourself)

| Claim | How to verify |
|---|---|
| Replay determinism | Run Tour twice and compare `viewmodel.hash` |
| Tier A truth protection | Confirm `tier_a_drops` is `0` in `metrics.json` |
| Share-safe export posture | Run export with `--share-safe`; inspect refusal report behavior |
| Constitutional alignment | Run `cargo test` (`docs_guard` enforces constitutional drift checks) |

## Core Workflows

### Determinism check (rerun hash)

```bash
cat tour-output/viewmodel.hash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir tour-output-rerun
cat tour-output-rerun/viewmodel.hash
```

Expected result: both hash files match.

### View an EventLog in TUI

```bash
cargo run -p panopticon-tui --bin panopticon -- view docs/assets/readme/sample-eventlog.jsonl
```

Run in a real interactive terminal (TTY).

### Showcase profile (visual demo mode)

```bash
cargo run -p panopticon-tui --bin panopticon -- \
  view docs/assets/readme/sample-eventlog.jsonl --profile showcase
```

`showcase` only changes presentation chrome and emphasis; truth ordering and proof semantics remain unchanged.

### Showcase gallery

Incident Lens (standard):

![Incident Lens](docs/assets/readme/incident-lens.svg)

Incident Lens (showcase):

![Incident Lens Showcase](docs/assets/readme/incident-lens-showcase.svg)

Forensic Lens (showcase):

![Forensic Lens Showcase](docs/assets/readme/forensic-lens-showcase.svg)

Truth HUD (showcase):

![Truth HUD Showcase](docs/assets/readme/truth-hud-showcase.svg)

### Export with share-safe checks

```bash
cargo run -p panopticon-tui --bin panopticon -- export \
  docs/assets/readme/sample-export-clean-eventlog.jsonl \
  --share-safe \
  --output out/bundle.tar.zst \
  --refusal-report out/refusal-report.json
```

### Robot mode for AI agents

Use machine-readable mode for automation:

```bash
cargo run -p panopticon-tui --bin panopticon -- \
  --json tour fixtures/large-stress.jsonl \
  --stress --output-dir tour-output
```

Behavior contract:

- `--json` returns compact structured output for success and errors.
- When stdout is piped, CLI auto-switches to JSON (unless `--human` is set).
- Error payloads include `code`, `message`, and `suggestions`.
- Parser authority is explicit: `clap` owns subcommand aliases and parse semantics.
- Normalization is bounded to known option spelling repairs and never rewrites positionals (including after `--`).

Force human-readable output even when piping:

```bash
cargo run -p panopticon-tui --bin panopticon -- \
  --human --help
```

Robot JSON contract keys (`schema_version=panopticon-cli-robot-v1.1`):

| Key | Type | Notes |
|---|---|---|
| `schema_version` | string | Contract version for parsers |
| `ok` | bool | Success/failure discriminator |
| `code` | string | Stable status code (`OK`, `INVALID_ARGS`, `NOT_FOUND`, `EXPORT_REFUSED`, `RUNTIME_ERROR`) |
| `message` | string | Human-readable summary |
| `suggestions` | array[string] | Actionable next commands or hints |
| `exit_code` | number | Process exit code mirror |
| `data` | object | Success payload (present on success envelopes) |
| `notes` | array[string] | Optional normalization notes when intent-repair was applied |

Exit codes:

- `0`: success
- `1`: not found
- `2`: invalid args
- `3`: export refused (share-safe scanner refusal)
- `4`: runtime error

## Architecture Snapshot

```mermaid
flowchart TD
    A[Agent Cassette JSONL] --> B[Importer]
    B --> C[Append Writer<br/>assigns commit_index]
    C --> D[EventLog JSONL + Blob Store]
    D --> E[Reducer]
    E --> F[Projection]
    F --> G[ViewModel]
    G --> H[Incident Lens + Forensic Lens + Truth HUD]
    D --> I[Tour stress harness]
    I --> J[metrics.json]
    I --> K[viewmodel.hash]
    I --> L[ansi.capture]
    I --> M[timetravel.capture]
```

Workspace crates:

- `crates/panopticon-core`: event schema, append writer, reducer, projection
- `crates/panopticon-import`: Agent Cassette importer
- `crates/panopticon-export`: bundle export and share-safe scanning
- `crates/panopticon-tour`: stress harness and proof artifact emission
- `crates/panopticon-tui`: CLI and terminal UI lenses

## Governance Docs

- `docs/CAPACITY_ENVELOPE.md`
- `docs/BACKPRESSURE_POLICY.md`
- `docs/UX_SCOPE.md`
- `docs/UX_MODALITY_MATRIX.md`
- `docs/UX_VISUAL_TONE.md`
- `docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md`
- `PLANS.md`
- `AGENTS.md`

## Product Docs

- `docs/product/positioning.md`
- `docs/product/messaging.md`
- `docs/product/go-to-market-checklist.md`
- `docs/product/business-model-v1.md`
- `docs/product/founder-one-pager.md`
- `docs/product/roadmap-30-60-90.md`

## Community and Security

- `CONTRIBUTING.md`: contribution expectations and report quality checklist
- `SUPPORT.md`: support channels and triage priorities
- `SECURITY.md`: private vulnerability reporting policy
- `docs/COMMUNITY_TRIAGE_PLAYBOOK.md`: maintainer triage and severity flow
- `.github/ISSUE_TEMPLATE/`: issue intake forms for bug and determinism reports
- `.github/pull_request_template.md`: PR evidence and risk template

## Status

Panopticon v0.1 implements the core truth pipeline. Release and public-facing documentation tracks continue in parallel.

Track current work:

- `.beads/issues.jsonl`
- `docs/README_LAUNCH_PLAN.md`
- `docs/RELEASE_PACKAGING_CHECKLIST.md`
- `docs/RELEASE_TRUST_VERIFICATION.md`

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Troubleshooting

### `--stress flag is required`

`tour` intentionally refuses non-stress mode in v0.1. Add `--stress`.

### `Export without secret scanning is not supported`

`export` requires `--share-safe`.

### Tour hash mismatch across reruns

Treat as determinism regression. Re-run on an idle machine, then inspect recent reducer/projection changes.

### `view` fails on fixture path

`view` expects EventLog JSONL, not Agent Cassette fixture JSONL.

## README Assets

Reference captures and visuals live under `docs/assets/readme/`:

- `docs/assets/readme/incident-lens.txt`
- `docs/assets/readme/incident-lens.svg`
- `docs/assets/readme/incident-lens-showcase.txt`
- `docs/assets/readme/incident-lens-showcase.svg`
- `docs/assets/readme/forensic-lens.txt`
- `docs/assets/readme/forensic-lens.svg`
- `docs/assets/readme/forensic-lens-showcase.txt`
- `docs/assets/readme/forensic-lens-showcase.svg`
- `docs/assets/readme/truth-hud-degraded.txt`
- `docs/assets/readme/truth-hud-degraded.svg`
- `docs/assets/readme/truth-hud-showcase.txt`
- `docs/assets/readme/truth-hud-showcase.svg`
- `docs/assets/readme/export-refusal.txt`
- `docs/assets/readme/architecture.mmd`

Refresh deterministically:

```bash
scripts/refresh_readme_assets.sh
```
