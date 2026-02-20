# A2 Hotspot Profiling Evidence (2026-02-17)

## Scope

Bead: `bd-2jw` (A2-2)

Goal: capture top hotspots by measured contribution, plus memory/I-O profile evidence, using available tooling in this environment.

## Profiling blocker (privileged sampling)

Privileged profilers were attempted and blocked by host policy:

- `perf stat -d ...` failed with `perf_event_paranoid=4`
- `strace -f -c ...` failed (`PTRACE_TRACEME/PTRACE_SEIZE: Operation not permitted`)

This environment does not allow kernel perf counters or ptrace sampling for unprivileged runs.

## Fallback method used (non-privileged, reproducible)

Stage-level timing profile was captured with deterministic in-process instrumentation in `vifei-tour` via:

```bash
VIFEI_TOUR_PROFILE_ITERS=12 cargo run -q -p vifei-tour --bin profile_tour --release
```

Memory/I-O envelope counters were captured with:

```bash
/usr/bin/time -v env VIFEI_TOUR_PROFILE_ITERS=12 \
  cargo run -q -p vifei-tour --bin profile_tour --release
```

## Results

### CPU hotspot ranking (stage share of end-to-end runtime)

From `profile_tour` run (`12` iterations):

- `reducer`: `82.85%`
- `parse_fixture`: `9.62%`
- `append_writer`: `7.50%`
- `metrics_emit`: `0.03%`
- `projection`: `~0%` (below printed precision)

Primary hotspot is clearly reducer execution.

### End-to-end distribution

From `profile_tour` run (`12` iterations):

- `p50`: `821.56ms`
- `p95`: `837.24ms`
- `p99`: `888.97ms`

### Allocation/I-O envelope (whole-process)

From `/usr/bin/time -v` wrapper around the same profile run:

- Max RSS: `39092 KB`
- File system inputs: `0`
- File system outputs: `8`
- Minor page faults: `18965`
- Major page faults: `0`

Interpretation:

- This workload is CPU-dominant, not disk-I/O bound.
- Memory envelope is stable and moderate for the tested fixture scale.

## Privileged profiling recipe (for richer sampling on capable hosts)

On a host where you can adjust sysctl and use perf:

```bash
# temporary (until reboot)
sudo sysctl -w kernel.perf_event_paranoid=1

# collect counters
perf stat -d cargo run -q -p vifei-tour --bin bench_tour --release

# collect sampled call stack profile
perf record -F 99 --call-graph dwarf -- \
  cargo run -q -p vifei-tour --bin bench_tour --release
perf report --stdio | head -n 120
```

Optional ptrace-based syscall profile:

```bash
strace -f -c cargo run -q -p vifei-tour --bin bench_tour --release
```

## Outcome for next bead

Hotspot evidence is sufficient to proceed to A2-3 (`bd-14f`) candidate ranking:

- focus performance candidates on reducer-path cost first,
- deprioritize projection/emit micro-optimizations for now.
