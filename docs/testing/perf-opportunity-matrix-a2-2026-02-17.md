# A2 Opportunity Matrix And Candidate Selection (2026-02-17)

## Inputs

- Baseline pack: `docs/testing/perf-baseline-a2-2026-02-17.md`
- Hotspot evidence: `docs/testing/perf-hotspots-a2-2026-02-17.md`

Key measured signal used for ranking:

- Tour stage hotspot shares: reducer `82.85%`, parse `9.62%`, append `7.50%`, projection/emit ~`0%`

## Scoring method

Score = `(Impact × Confidence) / Effort`

- Impact: expected movement on p95 latency/throughput for representative tour workload (1-5)
- Confidence: strength of current evidence + implementation tractability (1-5)
- Effort: expected engineering/test cost (1-5)

## Opportunity matrix

| Candidate | Evidence link | Impact | Confidence | Effort | Score |
|---|---|---:|---:|---:|---:|
| C1. In-place reducer path for replay/tour loops (avoid per-event `State` clone) | Reducer dominates at `82.85%` | 5 | 4 | 3 | 6.67 |
| C2. Stream fixture parsing (remove `read_to_string` full-buffer step) | Parse stage `9.62%` | 3 | 4 | 2 | 6.00 |
| C3. Tour-local append fast path (durability mode for temp EventLog in stress harness only) | Append stage `7.50%` | 3 | 3 | 4 | 2.25 |
| C4. Seek-point capture micro-optimizations inside reducer loop | Residual inside reducer stage, but likely small | 2 | 2 | 3 | 1.33 |
| C5. Projection/emit optimizations | Stage share near `0%` | 1 | 5 | 2 | 2.50 |

Selection policy used:

- prioritize high-score items that can be proven isomorphic against current deterministic contracts,
- avoid speculative work where measured stage share is negligible.

## Prioritized implementation shortlist (A2)

1. C1 (implement next): in-place reducer path for replay/tour loops.
2. C2 (implement after C1 if C1 gain is insufficient): stream fixture parsing.
3. C3 (defer unless needed): tour-local append fast path with strict scope guard.

## Candidate detail packs

### C1. In-place reducer path for replay/tour loops

- Lever: add a mutable reducer entrypoint (`reduce_in_place`) and use it in replay-heavy loops while preserving existing pure `reduce` API.
- Equivalence oracle:
  - For the same committed event sequence, final `State`, `state_hash`, `ViewModel`, and `viewmodel.hash` are byte-identical.
  - Tour artifacts (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`) remain identical.
- Isomorphism proof sketch:
  - `reduce` currently does `clone + deterministic mutation + return`.
  - `reduce_in_place` performs the same deterministic mutation directly on mutable state.
  - If mutation order and field writes are unchanged, resulting state graph is identical.
- Tests required:
  - replay parity test: old replay path vs new in-place path across fixtures.
  - hash parity test: `state_hash` and `viewmodel.hash` exact equality.
  - Tour artifact golden parity test.
- Regression guardrail:
  - `profile_tour` p95 threshold check in CI artifact lane (non-failing monitor first, then hard threshold after stabilization).
- Rollback plan:
  - retain old replay path behind a narrow fallback function; revert call sites to old path in one commit if parity/perf fails.

### C2. Stream fixture parsing

- Lever: replace full-file `read_to_string` in tour with streaming parse from `BufReader<File>`.
- Equivalence oracle:
  - Parsed import-event sequence must be byte-identical to current parse result for the same fixture.
- Isomorphism proof sketch:
  - Parsing grammar and record ordering remain unchanged; only input transport changes from buffered string to streaming reader.
- Tests required:
  - parser parity test between old and new path on canonical fixtures.
  - large-stress deterministic artifact parity test.
- Regression guardrail:
  - memory envelope (`Max RSS`) monitor in baseline refresh doc.
- Rollback plan:
  - keep old helper for one cycle; switch back if parity mismatches appear.

### C3. Tour-local append fast path (deferred)

- Lever: introduce explicit tour-only durability mode for temporary EventLog writes, without changing default append-writer durability semantics.
- Equivalence oracle:
  - committed event sequence and assigned `commit_index` values remain exactly unchanged.
- Isomorphism proof sketch:
  - durability policy changes persistence behavior only; event assignment/order logic stays identical.
- Tests required:
  - append parity test for commit_index and synthesized event insertion order.
  - no change to default mode tests.
- Regression guardrail:
  - explicit assertion that production/default writer still fsyncs Tier A.
- Rollback plan:
  - remove tour-only mode flag and revert to default writer behavior.

## Out-of-scope for this round

- projection/emit micro-optimizations: currently too small to move p95 materially.
- speculative algorithm swaps without direct hotspot evidence.

## Chosen next bead handoff

`bd-qx4` should implement C1 only (one performance lever), with proof-oriented parity tests and rollback-safe shape.
