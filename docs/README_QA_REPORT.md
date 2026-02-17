# README QA Report (`bd-1w9.5`)

Date: 2026-02-17

Scope:

- readability and structure sanity pass
- command validity checks for README examples
- asset/link integrity checks for referenced files

## Validation results

1. CLI help command

```bash
cargo run -p panopticon-tui --bin panopticon -- --help
```

Result: PASS. Command surface matches README sections (`view`, `export`, `tour`).

2. Deterministic stress Tour command

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/readme-tour-a
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/readme-tour-b
cat /tmp/readme-tour-a/viewmodel.hash /tmp/readme-tour-b/viewmodel.hash
```

Result: PASS. Hashes matched:

`000573091386a86cabe6935bbe997897a83f42cf89595238e55c2f9c8d45eda6`

3. Share-safe export command

```bash
cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output /tmp/readme-export/bundle.tar.zst --refusal-report /tmp/readme-export/refusal-report.json
```

Result: PASS. Bundle generated successfully.

4. TUI `view` command behavior in non-interactive environment

```bash
cargo run -p panopticon-tui --bin panopticon -- view docs/assets/readme/sample-eventlog.jsonl
```

Result: expected failure in this non-TTY environment (`Permission denied (os error 13)`). README already states `view` requires an interactive terminal.

5. Asset path checks

Verified README-referenced files exist:

- `docs/assets/readme/incident-lens.txt`
- `docs/assets/readme/forensic-lens.txt`
- `docs/assets/readme/truth-hud-degraded.txt`
- `docs/assets/readme/export-refusal.txt`
- `docs/assets/readme/architecture.mmd`

Result: PASS.

## Readability notes

- Header ordering is now consistent with first-time user flow (problem -> quickstart -> trust -> workflows -> architecture).
- Language remains evidence-first and avoids unverifiable claims.
- Badge count is restrained and purpose-driven.

## Follow-up

- Keep `README.md`, `docs/README_VERIFICATION.md`, and `docs/README_ASSET_PLAYBOOK.md` synchronized whenever CLI contract or asset generation changes.
