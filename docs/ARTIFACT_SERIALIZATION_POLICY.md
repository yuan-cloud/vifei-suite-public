# Artifact Serialization Policy (v0.1)

This policy defines stable output modes for Vifei Tour proof artifacts.

## Goal

Prevent accidental byte-shape drift in deterministic surfaces unless explicitly versioned and approved.

## Current policy

`vifei-tour` artifact modes:

- `metrics.json`: pretty JSON (`serde_json::to_string_pretty`)
- `timetravel.capture`: pretty JSON (`serde_json::to_string_pretty`)
- `viewmodel.hash`: plain text BLAKE3 hex, newline-terminated (`<64-hex>\n`)
- `ansi.capture`: deterministic ANSI text rendering

## Change control

Do not switch artifact serialization mode (pretty <-> compact, newline behavior, field ordering assumptions) silently.

If a mode change is necessary:

1. Add or update tests that assert the new byte-shape contract.
2. Document rationale in `docs/RISK_REGISTER.md`.
3. Update any dependent verification docs and tooling.
4. Treat the change as an explicit contract change, not a refactor.

## Non-goals

- This policy does not define UI styling.
- This policy does not change truth-path ordering (`commit_index`) or projection invariants.
