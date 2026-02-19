# A3 Post-C2 Hotspot Profile (2026-02-17)

## Bead

- `bd-2aum` (A3-2)

## Commands

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
/usr/bin/time -v env VIFEI_TOUR_PROFILE_ITERS=12 \
  cargo run -q -p vifei-tour --bin profile_tour --release
```

## Results

### Run distribution

- p50: `142.95ms`
- p95: `153.24ms`
- p99: `191.66ms`

### Hotspot shares

- parse fixture: `50.08%`
- append writer: `42.68%`
- reducer: `7.07%`
- projection: `~0%`
- metrics emit: `0.13%`

### Resource envelope (`/usr/bin/time -v` run)

- Max RSS: `38584 KB`
- File system inputs: `0`
- File system outputs: `16`
- Minor page faults: `17813`
- Major page faults: `0`

## Comparison to A2 closeout

A2 closeout baseline (`docs/testing/a2-closeout-report-2026-02-17.md`):

- p50 `153.21ms`, p95 `176.94ms`, p99 `207.24ms`

Post-C2 delta:

- p50 improved by ~`6.7%`
- p95 improved by ~`13.4%`
- p99 improved by ~`7.5%`

Interpretation:

- C2 delivered measurable latency gains.
- Top hotspots are now split between parse and append, with parse still slightly higher.
- Reducer remains de-risked as a major hotspot.
