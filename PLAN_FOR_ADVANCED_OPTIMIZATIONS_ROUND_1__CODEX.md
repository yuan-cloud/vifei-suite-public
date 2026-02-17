# PLAN_FOR_ADVANCED_OPTIMIZATIONS_ROUND_1__CODEX

## Scope

Round A only: baseline + hotspot identification + candidate ranking.
No behavior-changing optimizations are implemented in this round.

## Baseline Measurements (2026-02-17)

Environment:
- Repo: `/data/projects/PanopticonAliveca2.5`
- Fixture: `fixtures/large-stress.jsonl` (19,475 events)
- Command under test:
  - `cargo run -q -p panopticon-tui -- tour --stress fixtures/large-stress.jsonl --output-dir <dir>`

### Tour stress runtime distribution

Measured with 15 sequential runs (`/usr/bin/time -f '%e'`):
- n = 15
- mean = 3.425s
- p50 = 3.410s
- p95 = 3.600s
- p99 = 3.600s
- min = 3.170s
- max = 3.730s

### Single-run resource profile (`/usr/bin/time -v`)

Tour run:
- wall = 3.74s
- user = 3.56s
- system = 0.21s
- CPU = 101%
- max RSS = 47,520 KB

Full test suite (`cargo test`):
- wall = 16.32s
- user = 35.47s
- system = 2.66s
- CPU = 233%
- max RSS = 250,428 KB

## Profiling Availability and Limits

- `perf` exists on host, but sampling is blocked by `perf_event_paranoid=4`.
- Result: no function-level CPU percentage profile captured in this round.
- Action required for deeper hotspot proof: run with lower paranoid level or CAP_PERFMON in controlled environment.

## Current Hotspot Model (from measured runtime + code path analysis)

Primary path for Tour runtime:
1. JSONL parse (`parse_cassette`) over entire fixture
2. Append writer replay to EventLog JSONL
3. EventLog reread + reducer replay
4. Artifact serialization and writes (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`)

Most likely dominant costs:
1. Full JSON parse/import loop over 19k lines
2. Event replay/reduction pass over all committed events
3. Redundant parse/replay work inside some test invariants that reconstruct the pipeline

## Opportunity Matrix (Impact x Confidence / Effort)

Scoring legend: Higher is better.

1. Remove redundant parse/replay in Tour invariant tests
- Impact: Medium (test wall-time reduction)
- Confidence: High
- Effort: Low
- Score: High
- Notes: `tour_invariants` currently replays fixture for derivation checks; can share generated eventlog/artifacts per test run.

2. Add benchmark harness for Tour stages (parse, append, reduce, emit)
- Impact: Medium
- Confidence: High
- Effort: Low-Medium
- Score: High
- Notes: Enables targeted optimization instead of global guesses.

3. Reduce serialization overhead in artifacts where pretty JSON is unnecessary
- Impact: Low-Medium
- Confidence: Medium
- Effort: Low
- Score: Medium
- Notes: `metrics.json` / `timetravel.capture` currently pretty-printed. Could keep deterministic compact mode for CI artifacts.

4. Single-pass import/reduce pipeline for Tour (avoid parse->append->read split)
- Impact: Medium-High
- Confidence: Medium
- Effort: Medium-High
- Score: Medium
- Notes: Must preserve canonical append-writer semantics and determinism proof.

5. Large fixture optimization via binary fixture cache for tests
- Impact: Medium (CI time)
- Confidence: Medium
- Effort: Medium
- Score: Medium
- Notes: Higher maintenance risk; lower priority than structural test-speed wins.

## Isomorphism/Correctness Guardrails (required for every optimization)

For any accepted optimization bead:
1. Keep canonical ordering by `commit_index` unchanged.
2. Keep artifact bytes stable unless schema/version explicitly changed with justification.
3. Preserve all M7 invariant checks:
   - `tier_a_drops == 0`
   - ladder-order transition validity
   - `viewmodel.hash` rerun stability
   - `degradation_transitions` derivation from `PolicyDecision` events
4. Add/retain deterministic regression tests proving no behavioral drift.

## Proposed Implementation Sequence (for Round B/C)

1. B1: Add benchmark/trace points for Tour stage timings (parse/append/reduce/emit).
2. B2: Optimize invariant tests to eliminate duplicated parse/replay work.
3. B3: Re-measure; only then decide whether pipeline refactor is justified.
4. B4: If justified, implement minimal pipeline optimization with explicit equivalence tests.

## What to Bead Next (proposal only, not created yet)

- OPT-A1: Tour stage benchmark harness + baseline artifact
- OPT-A2: Test-path dedup optimization for `tour_invariants`
- OPT-A3: Optional compact artifact serialization mode (deterministic)
- OPT-A4: Re-measure and decide on pipeline-level refactor gate

## Recommendation

Proceed with low-risk, high-confidence optimizations first (benchmark instrumentation + test-path dedup).
Do not attempt deeper Tour pipeline refactor until stage-level timings confirm where wall time is concentrated.
