# TRACK-E Closeout: Failure-Contract Hardening (2026-02-18)

Bead: `bd-2cj9` (closeout via `bd-2cj9.5`)

## Scope and outcome

This closeout captures E1-E5 completion for robot-mode intent repair vs fail-closed execution hardening.

Outcome:
- Parser-level repair behavior remains intact (unambiguous normalization with explicit `notes`).
- Runtime/artifact integrity paths no longer silently downgrade serialization/write failures to placeholder defaults.
- A guardrail test now blocks reintroduction of the masking pattern class in runtime source paths.

## Before/after evidence

| Area | Before | After | Evidence |
|---|---|---|---|
| Audit visibility | No consolidated masking classification for recent commits | Explicit classification across last-5+HEAD with actionable follow-ups | `docs/testing/cli-masking-audit-2026-02-18.md` |
| Incident-pack artifact contract | Success contract checked manifest existence only | Success contract validates manifest file set, compare/replay schema keys, and non-placeholder JSON | `crates/vifei-tui/tests/cli_robot_mode_contract.rs` (`incident_pack_success_emits_manifest_and_hashes`) |
| Human failure clarity | Runtime output-dir failure behavior not asserted | Human-mode incident-pack runtime failure path is contract-tested for actionable guidance | `crates/vifei-tui/tests/cli_robot_mode_contract.rs` (`incident_pack_human_reports_runtime_error_when_output_dir_invalid`) |
| Delta tie-break masking | `serde_json::to_string(...).unwrap_or_default()` could silently collapse payload key segment | Explicit non-empty sentinel encoding on serialization error; no empty default fallback | `crates/vifei-core/src/delta.rs` |
| Reintroduction guard | No dedicated runtime masking regression guard | Runtime source scan guard test catches representative placeholder fallback reintroduction | `crates/vifei-core/tests/runtime_masking_guard.rs` |
| Policy clarity | Repair/fail boundary described but not matrixed | Two-layer parser vs execution matrix with explicit error-code mapping and test links | `AGENTS.md`, `docs/guides/CLI_DESIGN.md` |

## Fixes table

| Bead | Change | Status |
|---|---|---|
| `bd-2cj9.1` | Audit and classification of masking fallbacks | Closed |
| `bd-2cj9.2` | Strengthened incident-pack contract tests | Closed |
| `bd-2cj9.3` | Removed deterministic runtime masking fallback in `delta.rs` | Closed |
| `bd-2cj9.4` | Codified parser-repair vs execution-fail matrix in docs | Closed |
| `bd-2cj9.6` | Added runtime anti-masking guardrail test | Closed |

## Deferred/known tradeoffs

| Item | Reason deferred | Current containment |
|---|---|---|
| Exhaustive fault injection for every artifact write stage | Would increase harness complexity and runtime; not required for this pass | Contract tests cover representative runtime write failure and artifact schema integrity |
| Semantic AST-based masking detector | Current lightweight guard prioritizes low friction and immediate protection | String-window guard is bounded to runtime paths and synthetic reintroduction tests |
| Dedicated docs-lint for policy matrix drift | Existing tests already enforce behavior; docs drift tooling can be added later | Matrix references concrete test files and current contract test names |

## Final checklist

- [x] Parser normalization still allowed only for unambiguous repairs and reported via `notes`.
- [x] Ambiguous/invalid syntax still fails `INVALID_ARGS`.
- [x] Runtime write/serialize failures fail `RUNTIME_ERROR`; no placeholder success fallback introduced.
- [x] Share-safe scanner refusal path remains `EXPORT_REFUSED`.
- [x] Runtime masking guard catches synthetic reintroduction.
- [x] Full gates passed after E5 (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `ubs --staged`).

## Rollback notes

If rollback is required:
1. Revert guard test commit (`runtime_masking_guard.rs`) only if it blocks legitimate reviewed exceptions.
2. If reverting delta fallback hardening, re-open `bd-2cj9.3` and restore a non-masking strategy before release; do not restore silent empty-default behavior.
3. Keep policy matrix docs synchronized with whichever behavior is live; never allow docs to imply silent runtime fallback is acceptable.

## Gate evidence snapshot

- Workspace gates passed repeatedly during E1-E5:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - `ubs --staged` (exit 0 for each landed bead)
