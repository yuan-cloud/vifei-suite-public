# A2 C1 Optimization Proof · In-Place Reducer Path (2026-02-17)

## Bead

- `bd-qx4`
- Lever implemented: C1 from opportunity matrix (`reduce_in_place` for replay-heavy loops)

## Change summary

- Added `reduce_in_place(&mut State, &CommittedEvent)` in `vifei-core`.
- Kept existing `reduce(&State, &CommittedEvent) -> State` API for compatibility.
- Updated replay path to use in-place mutation (`replay_from`).
- Updated Tour reduction loop to use in-place mutation.
- Added parity test: `replay_matches_clone_based_reduce_path`.

## Equivalence oracle

For identical committed event sequences:

1. Final reducer `State` is byte-equivalent in serialized form.
2. `state_hash` remains identical.
3. Tour determinism/artifact tests remain green (`run_tour_determinism`, large fixture invariants).

## Isomorphism proof sketch

Previous behavior:

- For each event: `state_next = reduce(&state_prev, event)`
- `reduce` clones `state_prev`, then applies deterministic field updates.

New behavior:

- For each event: `reduce_in_place(&mut state, event)`
- Applies the same deterministic field updates directly on current mutable state.

Because transition logic and write order are unchanged, and because state evolution remains a pure fold over ordered `commit_index` events, resulting states and hashes are unchanged.

## Before/after measurement

Command (both before and after):

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
```

### Before (from `perf-hotspots-a2-2026-02-17.md`)

- `p50`: `821.56ms`
- `p95`: `837.24ms`
- `p99`: `888.97ms`
- reducer hotspot share: `82.85%`

### After (current)

- `p50`: `153.70ms`
- `p95`: `189.83ms`
- `p99`: `191.66ms`
- reducer hotspot share: `8.40%`

### Delta

- p50 improved by ~`81.3%`
- p95 improved by ~`77.3%`
- p99 improved by ~`78.4%`

Interpretation:

- The selected single lever moved the measured bottleneck significantly.
- Post-change, parse + append dominate; reducer is no longer the primary hotspot.

## Regression guardrails

- Keep parity test for replay clone-path equivalence.
- Keep Tour determinism and invariants tests green in CI.
- Keep `profile_tour` output as A2 evidence artifact to detect regressions in stage share.

## Rollback

- Revert call sites to `reduce` in `replay_from` and Tour loop.
- Keep `reduce_in_place` behind non-used API until parity/perf is re-evaluated.
