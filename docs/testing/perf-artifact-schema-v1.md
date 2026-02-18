# Perf Artifact Schema v1

Bead: `bd-1943`

This document defines deterministic schema and storage layout for replay benchmark trend tracking.

## Artifact files

- Bench artifact (single-run summary):
  - default path: `.tmp/perf/bench_tour_metrics.json`
  - override: `PANOPTICON_TOUR_BENCH_ARTIFACT`
- Trend log (append-only JSONL):
  - default directory: `.tmp/perf/trends`
  - override: `PANOPTICON_PERF_TREND_DIR`
  - concrete file: `bench_tour/<target_os>-<target_arch>.jsonl`

## Bench artifact schema (`panopticon-tour-bench-v1`)

Top-level fields:
- `schema_version: string`
- `stats: object`
- `command: object`

`stats` fields:
- `iters: usize` (must be `> 0`)
- `run_ms_p50`, `run_ms_p95`, `run_ms_p99`, `run_ms_mean`: `f64`
- `throughput_events_per_sec_p50`, `throughput_events_per_sec_p95`, `throughput_events_per_sec_p99`: `f64`
- `peak_rss_kib: Option<u64>`

`command` fields:
- `argv: Vec<String>`
- `invoked_as: Option<String>`
- `fixture_path: String`
- `fixture_bytes: u64`
- `fixture_line_count: u64`
- `package_version: String`
- `target_os: String`
- `target_arch: String`

Validator requirements:
- `schema_version == "panopticon-tour-bench-v1"`
- `stats.iters > 0`
- `command.fixture_path` non-empty

## Trend record schema (`panopticon-perf-trend-v1`)

Top-level fields:
- `schema_version: string`
- `metric_schema_version: string`
- `key: object`
- `stats: object` (same shape as bench artifact `stats`)

`key` fields:
- `benchmark: string` (currently `"bench_tour"`)
- `git_sha: Option<String>` (from `PANOPTICON_GIT_SHA`)
- `target_os: String`
- `target_arch: String`
- `package_version: String`
- `fixture_path: String`

Validator requirements:
- `schema_version == "panopticon-perf-trend-v1"`
- `metric_schema_version == "panopticon-tour-bench-v1"`
- `key.benchmark == "bench_tour"`
- `stats.iters > 0`

## Determinism guarantees

- Field ordering is declaration-order stable (Rust `struct` + `serde`).
- Trend storage is append-only JSONL for diff-friendly longitudinal review.
- Perf artifacts are advisory/derived diagnostics; they do not alter truth-path ordering, hashing, or EventLog semantics.
