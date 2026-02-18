# README QA Report (`bd-1w9.5`)

Date: 2026-02-18

Scope:

- readability and structure sanity pass
- command validity checks for README examples
- asset/link integrity checks for referenced files

## Validation results

Detailed command evidence lives in `docs/README_VERIFICATION.md`.

This QA report intentionally summarizes gate outcomes to avoid duplicating the full verification log.

1. CLI surface check

```bash
cargo run -p panopticon-tui --bin panopticon -- --human --help
```

Result: PASS (informational). Human-readable command surface matches README sections (`view`, `export`, `tour`).
Source: `docs/README_VERIFICATION.md` (Command Validation Matrix items 1-3).

2. Deterministic stress Tour command

```bash
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/readme-tour-a
cargo run -p panopticon-tui --bin panopticon -- tour fixtures/large-stress.jsonl --stress --output-dir /tmp/readme-tour-b
cat /tmp/readme-tour-a/viewmodel.hash /tmp/readme-tour-b/viewmodel.hash
```

Result: PASS. Hashes matched.
Source: `docs/README_VERIFICATION.md` (hash compare evidence).

3. Share-safe export command

```bash
cargo run -p panopticon-tui --bin panopticon -- export docs/assets/readme/sample-export-clean-eventlog.jsonl --share-safe --output /tmp/readme-export/bundle.tar.zst --refusal-report /tmp/readme-export/refusal-report.json
```

Result: PASS. Bundle generated successfully.
Source: `docs/README_VERIFICATION.md` (export success evidence).

4. TUI `view` command behavior in non-interactive environment

```bash
cargo run -p panopticon-tui --bin panopticon -- view docs/assets/readme/sample-eventlog.jsonl
```

Result: expected failure in non-TTY environments. README already states `view` requires an interactive terminal.
Source: `docs/README_VERIFICATION.md` (view smoke note).

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
- Treat `docs/README_VERIFICATION.md` as the canonical command evidence log for README claims.
