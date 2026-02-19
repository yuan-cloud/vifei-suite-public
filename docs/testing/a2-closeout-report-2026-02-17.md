# A2 Closeout Report · Product-Focused Perf + UX Proof (2026-02-17)

## Scope

Beads closed in this round:

- `bd-qhk` (baseline refresh)
- `bd-2jw` (hotspot profiling)
- `bd-14f` (opportunity matrix)
- `bd-qx4` (single-lever perf implementation)
- `bd-18m` (investigation UX audit)
- `bd-hov` (single-lever UX implementation)

## Executive result

A2 delivered a high-confidence speed improvement on the measured critical path and a targeted narrow-mode UX improvement, while preserving deterministic contracts and existing truth invariants.

## Before/after performance proof

Measurement command:

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
```

| Metric | Before (A2-2) | After (A2-7) | Delta |
|---|---:|---:|---:|
| p50 total | 821.56ms | 153.21ms | -81.4% |
| p95 total | 837.24ms | 176.94ms | -78.9% |
| p99 total | 888.97ms | 207.24ms | -76.7% |
| reducer stage share | 82.85% | 6.90% | hotspot removed |

Interpretation:

- The implemented C1 lever (in-place reducer fold) materially moved p95/p99.
- Post-change bottlenecks are now parse + append, not reducer.

## UX workflow gains

Validated via modality tests and refreshed deterministic readme artifacts:

- Incident Lens in narrow widths now preserves explicit `Next action:` guidance.
- Guidance remains deterministic and stable under width bucket checks.
- Desktop semantics remain intact.

Evidence:

- `docs/testing/ux-audit-a2-2026-02-17.md`
- `docs/testing/ux-improvement-hov-a2-2026-02-17.md`
- `docs/assets/readme/incident-lens.txt`

## Safety and determinism checks

Quality gates passed after changes:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

Proof-oriented checks added/kept:

- reducer parity test (`replay_matches_clone_based_reduce_path`)
- Tour determinism and invariants suites remain green
- modality contract checks remain green

## Hotspot closure status

- Closed: reducer clone overhead hotspot.
- Open (next candidates): parse fixture (`~51%`), append writer (`~41%`).

## Remaining risks

1. Performance evidence is fixture-based; broaden workload profiles before large architectural changes.
2. UX improvement validated by deterministic render tests; no human-study time-to-answer metrics yet.
3. Parse/append optimizations should continue one lever at a time with same oracle/rollback discipline.

## Release-note ready summary

- Improved tour profiling p95 latency by ~79% with deterministic-safe in-place reducer transitions.
- Removed reducer as dominant hotspot; parse/append are now top optimization targets.
- Improved narrow incident triage usability by preserving explicit next-step guidance under constrained widths.
- Preserved deterministic and constitutional contracts; full quality/test gates remain green.
