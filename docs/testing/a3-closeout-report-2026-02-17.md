# A3 Closeout Report · Parse/Append Optimization Round (2026-02-17)

## Scope

Beads covered:

- `bd-6wkf` (A3-1 C2 implementation)
- `bd-2aum` (A3-2 post-C2 profiling)
- `bd-3ufi` (A3-3 C3 decision gate)
- `bd-3vv0` (A3-4 conditional implementation, closed as no-go)
- `bd-141p` (A3-5 closeout)

## Executive summary

A3 delivered a deterministic-safe parse-path improvement (C2), verified equivalence, and measurable latency gains. Based on updated evidence, C3 append durability branching was explicitly deferred as not justified this round.

## What changed

1. Streaming parse path in Tour (`BufReader<File>`) replaced full-file buffering in runtime path.
2. Reader-mode equivalence proof test added (`stream_fixture_parse_matches_buffered_parse`).
3. Post-change hotspot/memory profile captured and compared to A2 closeout.
4. C3 decision gate executed with explicit no-go rationale.

## Before/after proof (A2 closeout → A3 post-C2)

Profile command:

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
```

| Metric | A2 closeout | A3 post-C2 | Delta |
|---|---:|---:|---:|
| p50 total | 153.21ms | 142.95ms | -6.7% |
| p95 total | 176.94ms | 153.24ms | -13.4% |
| p99 total | 207.24ms | 191.66ms | -7.5% |

Hotspot split after C2:

- parse `50.08%`
- append `42.68%`
- reducer `7.07%`

## Safety and determinism posture

- No truth-path ordering changes.
- No hash composition changes.
- Equivalence test confirms streamed vs buffered parse event-sequence parity.
- Full quality gates passed (`fmt`, `clippy`, `test`).

## C3 decision outcome

- **No-go in A3.**
- Append is significant but not dominant versus parse.
- Added branching complexity/proof burden in append durability path is not justified under current evidence.

## Remaining risks and next recommendation

1. Evidence remains fixture-centered; broaden representative workloads before deeper pipeline changes.
2. Parse remains top hotspot; next round should prioritize parse-only one-lever candidates with same proof discipline.
3. Keep decision-gate model: avoid implementation when hotspot dominance and ROI are ambiguous.

## Release-note summary

- Improved Tour latency further after A2 by implementing streaming fixture parsing with equivalence proof.
- Maintained deterministic guarantees and full test/lint pass.
- Applied evidence-based no-go to avoid over-engineering append durability branching in this cycle.
