# A2 Baseline Refresh · 2026-02-17

Purpose: refresh measured baseline data for product-focused optimization decisions.

## Commands

```bash
OUT_DIR=.tmp/a2-fastlane FASTLANE_MAX_SECONDS=1200 scripts/e2e/fastlane.sh
OUT_DIR=.tmp/a2-cli scripts/e2e/cli_e2e.sh

# 12-run latency sample
for i in $(seq 1 12); do
  /usr/bin/time -f '%e' cargo run -q -p vifei-tui --bin vifei -- \
    tour --stress fixtures/large-stress.jsonl --output-dir .tmp/a2-baseline/tour-run-$i
done

VIFEI_TOUR_BENCH_ITERS=12 cargo run -q -p vifei-tour --bin bench_tour --release
/usr/bin/time -v cargo run -q -p vifei-tour --bin bench_tour --release
/usr/bin/time -v cargo test
```

## Results

### Fastlane
- elapsed: `3s`
- output: `.tmp/a2-fastlane/run.jsonl`, `.tmp/a2-fastlane/summary.txt`

### CLI E2E
- pass
- output: `.tmp/a2-cli/run.jsonl`, `.tmp/a2-cli/summary.txt`

### Tour stress latency sample (12 runs)
- n: `12`
- mean: `3.180s`
- p50: `3.115s`
- p95: `3.557s`
- p99: `3.671s`
- min: `3.010s`
- max: `3.700s`
- raw samples: `.tmp/a2-baseline/tour_times_seconds.txt`

### bench_tour (release)
- `tour_bench_iters=12`
- `tour_run_ms_p50=850.20`
- `tour_run_ms_p95=951.66`
- `tour_run_ms_p99=1071.08`
- output: `.tmp/a2-baseline/bench_tour_release.txt`

### Resource snapshots
- bench_tour release (`/usr/bin/time -v`):
  - max RSS: `38816 KB`
  - wall: `8.77s`
  - user/system: `8.40s / 0.36s`
- cargo test (`/usr/bin/time -v`):
  - max RSS: `114972 KB`
  - wall: `16.19s`
  - user/system: `35.07s / 3.42s`

## Determinism anchors
- fastlane and cli-e2e runs passed with stable contract stages and artifact checks.
- this baseline is suitable for A2-2 hotspot profiling comparison and A2-5 before/after deltas.

## Next
- produce top 3-5 hotspot table with measured contribution (`A2-2`) before selecting optimization implementation (`A2-5`).
