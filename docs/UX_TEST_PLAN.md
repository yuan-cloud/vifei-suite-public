# UX Test Plan v0.1

## Purpose
Define a repeatable operator UX validation protocol for Vifei CLI/TUI flows, with measurable outcomes and direct conversion of findings into beads.

Scope aligns with:
- `docs/UX_SCOPE.md`
- `docs/UX_MODALITY_MATRIX.md`
- `docs/UX_VISUAL_TONE.md`

## Preconditions
- Build succeeds: `cargo build`
- Canonical fixtures exist (`fixtures/large-stress.jsonl`, `docs/assets/readme/*.jsonl`)
- E2E scripts available (`scripts/e2e/cli_e2e.sh`, `tui_e2e_interactive` test)

## Modality Profiles
- Desktop profile: `120x30` terminal
- Narrow profile: `72x22` terminal

Both profiles are required for each baseline pass.

## Operator Task Script
1. `first_run_orientation`
- Goal: Operator can identify active lens, Truth HUD fields, and quit controls.
- Command(s): `cargo run -p vifei-tui --bin vifei -- --help`, then interactive `view` flow via PTY harness.
- Pass condition: Lens identity and Truth HUD are visible, quit path works.

2. `incident_to_forensic_triage`
- Goal: Operator can move from Incident Lens triage to Forensic Lens investigation.
- Command(s): PTY interaction sequence (`Tab`, `j/k`, `Enter`, `q`).
- Pass condition: Forensic lens appears, timeline navigation and inspector expansion work, clean exit.

3. `trust_verification`
- Goal: Operator can verify stress proof artifacts and Tier A integrity signal.
- Command(s): `scripts/e2e/cli_e2e.sh` (`tour_stress` stage).
- Pass condition: `metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture` exist; `tier_a_drops=0`.

4. `share_safe_refusal_recovery`
- Goal: Operator can understand refusal cause and exact next action.
- Command(s): `scripts/e2e/cli_e2e.sh` (`export_refusal` stage).
- Pass condition: stderr includes refusal reason and "Likely cause" guidance.

## Scoring Rubric
Per task, record:
- `status`: `pass` | `fail` | `skip`
- `completion_seconds`: elapsed wall time for task (or `0` for skipped)
- `error_type`: one of `none`, `cli_contract`, `tui_contract`, `artifact_missing`, `pty_env`, `docs_gap`, `other`
- `confidence`: integer `1-5`

Session-level score:
- `pass_rate = passed_tasks / runnable_tasks`
- `confidence_avg = mean(confidence)`
- `overall = GREEN` if `pass_rate=1.0` and `confidence_avg>=4.0`; otherwise `YELLOW`/`RED` by operator judgment.

## Error Taxonomy
- `cli_contract`: expected command UX contract changed (help/refusal text missing)
- `tui_contract`: expected lens/HUD/navigation behavior missing
- `artifact_missing`: required tour/export artifact absent
- `pty_env`: PTY capability not available (skip with explicit reason)
- `docs_gap`: operator could not complete without external clarification
- `other`: any uncategorized issue

## Run Template
Use this markdown skeleton for each pass:

```markdown
## UX Baseline Run <date>

### Environment
- Host:
- Rust/Cargo:
- Terminal profiles: desktop=<WxH>, narrow=<WxH>

### Task Results
| Task | Profile | Status | Completion (s) | Error type | Confidence (1-5) | Evidence |
|---|---|---|---:|---|---:|---|
| first_run_orientation | desktop | pass |  | none |  |  |
| incident_to_forensic_triage | desktop | pass/skip/fail |  |  |  |  |
| trust_verification | desktop | pass |  | none |  |  |
| share_safe_refusal_recovery | desktop | pass |  | none |  |  |
| incident_to_forensic_triage | narrow | pass/skip/fail |  |  |  |  |

### Summary
- pass_rate:
- confidence_avg:
- overall:

### Findings -> Beads
- [BUG] <finding> -> <bead-id>
- [UX] <finding> -> <bead-id>
- [DOCS] <finding> -> <bead-id>
```

## Finding Conversion Rule (Mandatory)
For every non-trivial `fail` or repeated `skip`, create a bead:

```bash
br create "<TAG>: <concise finding title>" -t <bug|task> -p <P0..P3> -l testing,ux -d "<repro + impact + evidence path>"
```

Tag prefixes:
- `BUG` for product behavior defects
- `UX` for discoverability/readability/task-flow gaps
- `DOCS` for missing/unclear operator guidance

## Baseline Requirement
At least one completed baseline run must be committed with evidence paths and converted findings.
