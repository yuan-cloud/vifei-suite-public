# CORE Truth-Path Split Preflight (Plan-Only)

Status: plan-only. No `vifei-core` code moves are performed by this document.

## Scope
- Target area: `crates/vifei-core/src/event.rs`, `crates/vifei-core/src/reducer.rs`, `crates/vifei-core/src/projection.rs`
- Goal: prepare a safe future decomposition path without changing behavior, ordering, or artifact contracts.

## Constitutional Constraints
- Capacity and overload behavior: `docs/CAPACITY_ENVELOPE.md`
- Backpressure ladder and projection invariants: `docs/BACKPRESSURE_POLICY.md`

This preflight is constrained by those documents. Any future split must preserve those contracts exactly.

## Invariant-Risk Map

1. Canonical ordering risk
- Risk: accidental ordering drift if event/reducer code is split with hidden ordering assumptions.
- Guardrail: keep `commit_index` ordering checks in `eventlog`/reducer tests green; add targeted regression test for replay order equivalence before and after split.

2. Hash contract risk (`state_hash`, `viewmodel.hash`)
- Risk: moving serialization/hashing helpers could change input ordering or included fields.
- Guardrail: require byte-stability tests and 10-run determinism tests to pass unchanged; keep `BTreeMap`/sorted iteration discipline.

3. Projection invariant drift risk
- Risk: module boundary refactors may unintentionally move or alter projection-level policy logic.
- Guardrail: preserve `ProjectionInvariants` API and behavior, plus `tour` artifact tests (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`).

4. API surface risk
- Risk: downstream crates (`import`, `export`, `tour`, `tui`) break if `vifei-core` public items move without re-export strategy.
- Guardrail: phase with `pub use` compatibility shims in `lib.rs` until downstream imports are updated and validated.

5. Merge-conflict risk
- Risk: large-file decomposition touches hot files with high change frequency.
- Guardrail: one file family per bead, short-lived branches, full gates per bead, no mixed semantic edits.

## Dependency-Ordered Execution Plan (Future Beads)

1. Core split precheck bead (no moves)
- Snapshot baseline for `cargo test`, determinism tests, and core API usage sites.

2. `event.rs` internal decomposition (move-only)
- Split by concerns (wire types, payload helpers, serialization tests) with stable `pub` surface.

3. `reducer.rs` internal decomposition (move-only)
- Split state model, transition logic, checkpoint helpers; keep reducer outputs and `state_hash` behavior unchanged.

4. `projection.rs` internal decomposition (move-only)
- Split invariants model, viewmodel assembly, hash input path; preserve `viewmodel.hash` semantics.

5. Cleanup bead
- Remove temporary compatibility shims only after all downstream crates compile/test cleanly.

## Required Test Guardrails for Every Future Core Split Bead

- Mandatory gates:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- Determinism/contract focus:
  - core reducer/projection determinism tests (including repeated-run stability)
  - tour invariants tests (artifact shape and hash stability)
  - CLI robot contract tests (to catch accidental output drift downstream)

## Rollback Strategy

- Rollback unit: one bead/commit at a time.
- If any guardrail fails:
  1. revert only the failing split commit,
  2. restore green baseline,
  3. reopen bead with specific blocker notes.
- Do not stack additional core split work on top of a failing phase.

## Exit Criteria for Preflight

- This document exists and is linked from planning context.
- Future core split sequence is explicitly ordered and test-gated.
- No `vifei-core` truth-path code was modified in this bead.
