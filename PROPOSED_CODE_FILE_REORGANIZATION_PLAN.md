# Proposed Code File Reorganization Plan

Status: plan-only. No file moves or module splits are executed in this document.

## 1) Scope and intent

This plan targets structural clarity, onboarding speed, and safer long-term maintenance.
It focuses on no-brainer improvements first, with minimal disruption and explicit rollback points.

Goals:
- Keep crate-level boundaries stable and intuitive.
- Reduce single-file cognitive load in oversized modules.
- Improve discoverability for new contributors and coding agents.
- Avoid deep nesting and avoid broad churn.

Non-goals:
- No architecture rewrite.
- No behavior changes in truth-path logic.
- No constitutional content duplication.

## 2) Current-state assessment

The top-level crate structure is already strong:
- `panopticon-core`: truth model, reducer, projection, event log
- `panopticon-import`: cassette ingestion
- `panopticon-export`: share-safe export + scanner
- `panopticon-tour`: deterministic stress harness
- `panopticon-tui`: CLI + terminal UI lenses

Primary pain points are file size and mixed concerns within single files, not folder chaos.

Largest files (approximate line counts):
- `crates/panopticon-core/src/projection.rs` (~1698)
- `crates/panopticon-core/src/reducer.rs` (~1262)
- `crates/panopticon-core/src/event.rs` (~1044)
- `crates/panopticon-export/src/lib.rs` (~1361)
- `crates/panopticon-tour/src/lib.rs` (~775)
- `crates/panopticon-tui/src/main.rs` (~768)
- `crates/panopticon-tui/src/forensic_lens.rs` (~883)

Conclusion:
- Directory layout is acceptable.
- Internal module decomposition should be the first optimization target.

## 3) No-brainer reorganization candidates

### A. `panopticon-export` decomposition (highest ROI)

Current issue:
- `crates/panopticon-export/src/lib.rs` contains multiple concerns:
  - export orchestration
  - refusal report models and formatting
  - deterministic bundling helpers
  - extensive inline tests

Proposed structure:
- `crates/panopticon-export/src/lib.rs` (public API surface + re-exports only)
- `crates/panopticon-export/src/export_pipeline.rs` (run_export orchestration)
- `crates/panopticon-export/src/refusal_report.rs` (report schema + serialization)
- `crates/panopticon-export/src/bundle.rs` (tar/zstd deterministic packaging)
- keep `crates/panopticon-export/src/scanner.rs` as-is

Why:
- Fastest onboarding win for maintainers.
- Lower merge-conflict pressure during parallel edits.
- Cleaner test targeting by concern.

### B. `panopticon-tour` decomposition (high ROI)

Current issue:
- `crates/panopticon-tour/src/lib.rs` mixes pipeline orchestration and artifact rendering logic.

Proposed structure:
- `crates/panopticon-tour/src/lib.rs` (public types + orchestration entrypoints)
- `crates/panopticon-tour/src/artifacts.rs` (metrics/hash/capture writers)
- `crates/panopticon-tour/src/metrics.rs` (metrics structs + builders)
- `crates/panopticon-tour/src/pipeline.rs` (import/append/reduce/project sequence)

Why:
- Makes deterministic artifact contract easier to inspect and evolve safely.

### C. `panopticon-tui` decomposition (targeted)

Current issue:
- `crates/panopticon-tui/src/main.rs` has CLI parsing, envelope formatting, output mode logic, and command execution together.

Proposed structure:
- `crates/panopticon-tui/src/main.rs` (thin CLI entry)
- `crates/panopticon-tui/src/cli_contract.rs` (JSON/human envelope types and helpers)
- `crates/panopticon-tui/src/cli_commands.rs` (view/export/tour command runners)
- `crates/panopticon-tui/src/arg_normalization.rs` (intent-repair normalization logic)

Why:
- Reduces risk when updating robot-mode contract guarantees.
- Keeps behavior-critical surface testable in isolation.

## 4) Files that should stay as they are (for now)

- `panopticon-import/src/cassette.rs`
  - Large but cohesive, single-source mapping logic.
- `panopticon-core` large files (`event.rs`, `reducer.rs`, `projection.rs`)
  - Candidate for future split, but currently high-coupling truth-path code.
  - Should be split only with dedicated invariant-focused beads and heavy regression gates.

## 5) Calling-code impact map

For each candidate split, expected updates:

- `mod` declarations and `pub use` surfaces in crate `lib.rs`.
- Internal `use crate::...` paths after module extraction.
- Test imports that currently rely on same-file visibility.
- No external crate API break is expected if `pub` surface is preserved via re-exports.

Risk controls:
- Keep old public type/function names intact.
- Prefer move-only refactors before semantic edits.

## 6) Migration sequence (safe order)

### Phase 0: preflight safeguards
- Ensure all quality gates pass:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- Keep or add lightweight docs/link guards to prevent public-surface drift.

### Phase 1: export crate internal split
- Extract models/helpers first.
- Keep behavior unchanged.
- Run full gates.

### Phase 2: tour crate internal split
- Extract artifact emitters and metrics builders.
- Run full gates + tour determinism tests.

### Phase 3: tui CLI internal split
- Move contract/output-mode logic into dedicated modules.
- Run full gates + CLI contract integration tests.

### Phase 4: optional core decomposition planning
- Only after prior phases are stable and reviewed.
- Create a dedicated plan-only bead before any core truth-path split.

## 7) Rollback strategy

If a phase causes instability:
- Revert that phase commit only.
- Keep prior phases if green.
- Reopen bead with explicit blocker notes.
- Do not proceed to next phase until gates are clean.

## 8) Visibility and “private strategy” handling

Public docs should stay product-first, factual, and operator-focused.

Private strategy notes (hiring narrative, self-promotion copy experiments, audience positioning variants) should be kept out of public-facing docs and tracked separately, for example:
- local ignored path (such as `.private/`) for personal notes, or
- a separate private planning repo.

This separation keeps public credibility high and avoids awkward tone drift in user-facing materials.

## 9) Recommendation

Proceed with phased internal module decomposition; do not perform broad folder moves.
Start with `panopticon-export` and `panopticon-tour`, where clarity gains are highest and correctness risk is manageable.
