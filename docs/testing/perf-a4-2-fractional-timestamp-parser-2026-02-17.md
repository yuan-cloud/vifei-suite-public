# A4-2 Fractional Timestamp Parser Optimization (2026-02-17)

## Bead

- `bd-qhgs`

## Lever

Single lever only:

- Replace fractional-second parsing in `parse_iso8601_ns` with a zero-allocation digit loop (`parse_fractional_ns`) in `crates/vifei-import/src/cassette.rs`.

## Equivalence posture

Behavior preserved:

- same ISO-8601 subset acceptance/rejection semantics,
- same truncation semantics for >9 fractional digits,
- same right-padding semantics for <9 fractional digits,
- same fallback behavior (`fractional=0`) for invalid fractional content.

Validation:

- new parser edge-case tests added for truncate/pad/invalid-fraction cases,
- existing importer unit/integration tests remain green,
- full workspace quality gates remain green.

## Measurement

Commands:

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
/usr/bin/time -v env VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
```

Post-change runs:

- Run A: p50 `125.34ms`, p95 `143.86ms`, p99 `172.66ms`
- Run B: p50 `129.10ms`, p95 `141.07ms`, p99 `169.77ms`

Hotspot split (Run B):

- parse `42.14%`
- append `48.99%`
- reducer `8.68%`

Compared to A4-1 run B (`parse 44.62%`, `append 47.17%`):

- parse share reduced further by ~2.5 points,
- append remains the dominant hotspot.

## Resource envelope (`/usr/bin/time -v`, Run B)

- Max RSS: `38764 KB`
- File outputs: `8`
- Major faults: `0`

## Next implication

- Parse-stage pressure continues to trend down with deterministic-safe single-lever changes.
- Next candidate should remain append-focused, with explicit proof and no default durability semantic drift.
