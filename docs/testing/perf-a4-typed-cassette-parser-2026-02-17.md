# A4 Typed Cassette Parser Optimization (2026-02-17)

## Bead

- `bd-xx6w`

## Lever

Single lever only:

- Switch importer parse path from generic `serde_json::Value` map lookups to typed `CassetteRecord` deserialization in `crates/vifei-import/src/cassette.rs`.

## Equivalence posture

Behavior preserved:

- same source-order semantics,
- same event-id fallback behavior,
- same timestamp parse behavior,
- same tier/payload mapping behavior,
- same parse-error emission behavior.

Validation:

- existing importer unit/integration tests pass,
- downstream Tour determinism/invariants tests pass.

## Measurement

Command:

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
```

Latest runs after change:

- Run A: p50 `122.82ms`, p95 `144.95ms`, p99 `260.72ms`
- Run B: p50 `129.73ms`, p95 `142.57ms`, p99 `157.66ms`

Hotspot split (Run B):

- parse `44.62%`
- append `47.17%`
- reducer `8.03%`

Compared to A3 post-C2 baseline (`p95=153.24ms`, parse `50.08%`):

- p95 improved in both observed post-change runs,
- parse share reduced by ~5 percentage points,
- append is now the dominant stage.

## Resource envelope (`/usr/bin/time -v`, Run B)

- Max RSS: `38648 KB`
- File outputs: `24`
- Major faults: `0`

## Next implication

- This round successfully moved parse pressure down.
- Next single-lever candidate should target append stage (with the same decision-gate discipline).
