# PROPOSED CODE FILE REORGANIZATION PLAN

Status: plan-only. No file moves, renames, API changes, or behavior changes are executed in this document.

## 1) Purpose

This plan adapts your request to the actual Vifei Suite structure.

Key finding up front:
- The repo is **not** suffering from top-level folder chaos.
- The main risk is **oversized, mixed-concern files inside otherwise good crate boundaries**.

So the right move is targeted internal module decomposition, not broad directory churn.

## 2) Current architecture (what exists today)

Workspace crates are already clear and product-aligned:
- `crates/vifei-core`: truth model, append writer, reducer, projection, hashing
- `crates/vifei-import`: adapter ingestion and importer contracts
- `crates/vifei-export`: discovery, secret scanning, refusal reports, bundle output
- `crates/vifei-tour`: deterministic replay/tour pipeline and artifact generation
- `crates/vifei-tui`: interactive viewer, lenses, CLI entry/handlers

Largest current files (high cognitive-load candidates):
- `crates/vifei-core/src/projection.rs` (~1698)
- `crates/vifei-core/src/reducer.rs` (~1331)
- `crates/vifei-tui/src/cli_handlers.rs` (~1293)
- `crates/vifei-export/src/lib.rs` (~1180)
- `crates/vifei-core/src/event.rs` (~1044)
- `crates/vifei-tui/src/forensic_lens.rs` (~928)

## 3) Dependency and execution-flow map (condensed)

### A) CLI path
- `vifei-tui/src/main.rs`
- `vifei-tui/src/cli_handlers.rs`
- Depends on `vifei-core`, `vifei-import`, `vifei-export`, `vifei-tour`

### B) Truth path
- Importers produce `ImportEvent` (no canonical `commit_index`)
- `vifei-core/src/eventlog.rs` append writer assigns canonical `commit_index`
- `vifei-core/src/reducer.rs` and `projection.rs` produce deterministic state/viewmodel hashes

### C) Export safety path
- `vifei-export/src/lib.rs` orchestrates discovery + scanning + refusal report + bundle creation
- Secret scanning and refusal reporting are safety-critical and currently concentrated in one large file

### D) Tour/evidence path
- `vifei-tour/src/lib.rs` runs deterministic import→append→reduce→project flow
- Emits artifacts consumed by README/demo validation and trust proofs

## 4) Structural pain points

1. High-change files are too large (`cli_handlers.rs`, `export/lib.rs`), increasing merge collisions and review friction.
2. Mixed concerns inside single files slow onboarding (command routing + file I/O + contract formatting in one place).
3. Some crates already have partial decomposition (`tour` has `artifacts.rs`/`metrics.rs`), so consistency can improve further.

## 5) Reorganization strategy

Principle:
- Keep crate boundaries and public APIs stable.
- Refactor internally by concern in small, reversible phases.
- One structural lever per bead.

### Phase 1 (highest ROI, lowest behavior risk): `vifei-export`

Current concern mix inside `crates/vifei-export/src/lib.rs`:
- config/result surface types
- export pipeline orchestration
- refusal-report model and serialization
- path-label/share-safe helpers
- test block

Proposed internal structure:
- `src/lib.rs`: public API surface, crate docs, re-exports only
- `src/export_pipeline.rs`: `run_export` orchestration flow
- `src/refusal_report.rs`: refusal report schema/build/serialize helpers
- `src/path_label.rs`: share-safe path labeling helpers
- Keep `src/discover.rs`, `src/scanner.rs`, `src/secret_scan.rs`, `src/bundle.rs`

Why this first:
- Directly improves maintainability of security-sensitive code paths.
- Reduced chance of accidental regressions from unrelated edits.

### Phase 2: `vifei-tui` CLI internals

Current concern mix inside `crates/vifei-tui/src/cli_handlers.rs`:
- command orchestration
- JSON envelope builders
- compare/replay/export/tour handlers
- file output and report shaping

Proposed internal structure:
- `src/cli_handlers/mod.rs`: public dispatch and shared helper imports
- `src/cli_handlers/view.rs`
- `src/cli_handlers/compare.rs`
- `src/cli_handlers/export.rs`
- `src/cli_handlers/tour.rs`
- `src/cli_handlers/incident_pack.rs`
- `src/cli_handlers/envelope.rs`

Why this second:
- Largest operational surface for human + agent users.
- Makes behavior and tests easier to map to command families.

### Phase 3: `vifei-tour` pipeline clarity

Current `crates/vifei-tour/src/lib.rs` still mixes orchestration + state transitions + profile output.

Proposed internal structure:
- `src/lib.rs`: public types/re-exports
- `src/pipeline.rs`: deterministic run pipeline
- `src/profile.rs`: stage profiling and summary
- Keep `src/artifacts.rs`, `src/metrics.rs`

Why this third:
- Improves reproducibility auditing and benchmark readability.

### Phase 4 (only after earlier phases): selective `vifei-core` extraction

Core is safety-critical and coupled to constitutional invariants. Split only with strict proof steps.

Candidate extractions:
- `projection.rs`: split hash materialization helpers from view construction
- `reducer.rs`: split checkpointing code from event-reduce logic

Guardrail:
- No semantic edits in same commit as file moves.

## 6) Calling-code impact map (what must change during each phase)

For every extraction/move:
1. Update module declarations (`mod ...`) and visibility (`pub(crate)`/`pub`).
2. Update all `use crate::...` paths.
3. Keep public symbols stable via re-exports from `lib.rs`.
4. Update affected unit/integration tests to new module paths.
5. Re-run proof/contract tests before proceeding to next phase.

## 7) Safe migration workflow per phase

For each phase, execute in this order:
1. Add bead and mark in progress.
2. Move code mechanically (no behavior edits).
3. Compile and fix imports.
4. Run quality gates:
   - `cargo fmt --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
5. Run targeted contract/e2e tests for touched area.
6. Run `ubs --staged`.
7. Append risk entry in `docs/RISK_REGISTER.md`.
8. Commit with bead ID in subject.

## 8) Rollback plan

If a phase fails quality/contract checks:
- Revert only that phase commit.
- Keep previous green phases.
- Record blocker in bead notes.
- Do not start next phase until current one is stable.

## 9) What should NOT be reorganized now

Avoid these high-risk moves in current cycle:
- Broad renames across all crates at once
- Deep nested directory trees that reduce discoverability
- Simultaneous semantic refactor + structural move in same commit
- Core truth-path decomposition before smaller crates are stabilized

## 10) Recommended first implementation bead sequence

1. `EXPORT-MOD-1`: split `vifei-export/src/lib.rs` into pipeline + refusal + label modules.
2. `CLI-MOD-1`: split `vifei-tui/src/cli_handlers.rs` by command family.
3. `TOUR-MOD-1`: split `vifei-tour/src/lib.rs` orchestration/profile concerns.
4. `CORE-MOD-PLAN-1`: plan-only doc for safe core decomposition; no code movement yet.

## 11) Expected outcome

After these phases:
- Faster onboarding for humans and coding agents.
- Smaller review units and fewer merge collisions.
- Better isolation of security-sensitive and determinism-sensitive logic.
- No user-visible behavioral drift when done with mechanical-first commits and full gate coverage.
