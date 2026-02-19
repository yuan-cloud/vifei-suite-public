# CLI/Runtime Masking Audit (2026-02-18)

Bead: `bd-2cj9.1`

Scope:
- Last 5 commits plus current HEAD planning commits:
  - `aac0981` compare contract
  - `fecb7f3` incident-pack
  - `bf43ccd` fail-closed hardening patch
  - `2f6e91d` TRACK-E bead graph
  - `33adcff` TRACK-E audit start marker
  - `b152b2e` TRACK-E sequencing optimization

Objective:
- Detect fallback patterns that can hide real runtime/artifact regressions.
- Classify each finding as `INTENTIONAL_SAFE`, `RISKY_MASKING`, or `FALSE_POSITIVE`.

## Method

Evidence commands:

```bash
git log --oneline -n 6
rg -n "unwrap_or_default\\(|unwrap_or_else\\(|failed to serialize|RUNTIME_ERROR|incident-pack" \
  crates/vifei-tui/src/cli_handlers.rs \
  crates/vifei-core/src/delta.rs \
  crates/vifei-tour/src/bin/bench_tour.rs \
  crates/vifei-tui/tests/cli_robot_mode_contract.rs
git show --patch --stat fecb7f3 -- crates/vifei-tui/src/cli_handlers.rs
git show --patch --stat bf43ccd -- crates/vifei-tui/src/cli_handlers.rs
```

Classification rule:
- `RISKY_MASKING`: fallback can preserve success path while silently degrading correctness artifacts.
- `INTENTIONAL_SAFE`: fallback is dev ergonomics or display-only and cannot mask truth/artifact correctness.
- `FALSE_POSITIVE`: grep match not semantically related to masking.

## Findings Table

| ID | Location | Classification | Why |
|---|---|---|---|
| F1 | `fecb7f3` in `crates/vifei-tui/src/cli_handlers.rs` (`incident-pack` delta/replay writes used `serde_json::to_vec_pretty(...).unwrap_or_else(|_| b"{}".to_vec())`) | `RISKY_MASKING` | Serialization failure could write `{}` and continue, hiding artifact-shape regressions behind apparent success. |
| F2 | `bf43ccd` in `crates/vifei-tui/src/cli_handlers.rs` (`write_json_pretty(...)` + `RUNTIME_ERROR` return paths) | `INTENTIONAL_SAFE` | Correct fail-closed replacement; error is surfaced with structured runtime failure envelope. |
| F3 | `crates/vifei-core/src/delta.rs:126` (`serde_json::to_string(&event.payload).unwrap_or_default()`) | `RISKY_MASKING` | If payload serialization ever fails, tie-break key degrades to empty payload segment, potentially collapsing ordering distinctions in deterministic diff selection. |
| F4 | `crates/vifei-tour/src/bin/bench_tour.rs:190` (`env var parse fallback to default output path`) | `INTENTIONAL_SAFE` | Operational default for benchmark artifact path; does not affect truth path, event ordering, or production runtime correctness. |
| F5 | `crates/vifei-tui/src/cli_handlers.rs:402` (`unwrap_or_else(|| format!(\"event:{}\", ...))`) | `INTENTIONAL_SAFE` | Human-readable label fallback for refusal messages; no effect on canonical truth or deterministic artifact contents. |
| F6 | Planning commits `2f6e91d`, `33adcff`, `b152b2e` touching `.beads/issues.jsonl` | `FALSE_POSITIVE` | Bead metadata only; no runtime code path. |
| F7 | `crates/vifei-tui/src/cli_handlers.rs` compare command path (`build_compare_contract`, `emit_compare_contract_human`) | `INTENTIONAL_SAFE` | Reviewed current compare flow; parse/build failures already return structured `RUNTIME_ERROR` and no placeholder artifact write fallback remains in this path. |

## Commit-Level Conclusion

1. The concrete high-risk runtime masking bug was introduced in `fecb7f3` and already corrected in `bf43ccd`.
2. One adjacent masking risk remains in deterministic diff tie-break path (`delta.rs` payload serialization fallback).
3. Remaining fallback matches in scanned range are operational defaults or display-only fallbacks, not truth/artifact masking paths.

## Required Follow-up

- `bd-2cj9.2` (tests first): add contract checks that would fail if compare/replay artifacts regress to placeholders while command reports success.
- `bd-2cj9.3` (runtime hardening): resolve `delta.rs` tie-break serialization fallback (`F3`) using a non-masking strategy and explicit test evidence.
- `bd-2cj9.4` (policy codification): codify parser-repair vs execution-fail boundary to prevent reintroduction.
