# Capacity envelope · v0.1 targets

This document is constitutional.
It contains target values only in the `TARGET` columns (numeric thresholds and pinned constants).
Notes are explanatory only and must not introduce new policy.

All values are `TARGET` until `vifei tour --stress` measures and we promote them to `MEASURED`.

This doc is the single source of truth for thresholds and budgets.
Threshold labels here are API-level names; other docs and tests should reference labels, not copy values.

<!-- DOCS_GUARD:BEGIN CAPACITY_TABLES -->

## Throughput budgets

| Budget | TARGET | Notes |
|---|---:|---|
| Tier A events per second (NORMAL) | 2_000 | Import + Tour must keep up without drops |
| Tier A events per second (STRESS) | 10_000 | Tour stress target |

## Storage thresholds

| Threshold | TARGET | Notes |
|---|---:|---|
| Inline payload max bytes | 16_384 | Bytes are UTF-8 bytes of the inline payload as stored (no pretty printing). Above this, store as blob and reference by `payload_ref` |
| Max blob bytes | 50_000_000 | Bytes are the raw blob file size on disk. Above this, exporter refuses unless explicitly allowed |
| Tier A fsync interval events | 1 | 1 means fsync per Tier A append. v0.1 default is safer than faster |

## Timeouts and tolerances

| Setting | TARGET | Notes |
|---|---:|---|
| Clock skew tolerance ns | 50_000_000 | If a source moves backward by more than this, emit `ClockSkewDetected` |
| Checkpoint interval events | 5_000 | Reducer writes a checkpoint every N events |
| SQLite reindex max seconds (NORMAL) | 2 | Cache rebuild budget |

## Tour fixtures

| Fixture class | TARGET | Notes |
|---|---:|---|
| Small fixture events | 1_000 | For fast unit and snapshot tests |
| Large fixture events | 10_000 | For Tour stress and CI proof loop |

## UI budgets

| UI budget | TARGET | Notes |
|---|---:|---|
| Projection max ms per frame (NORMAL) | 10 | Projection must stay under this budget |
| Projection max ms per frame (STRESS) | 33 | Under stress, degrade projection, not Tier A ingestion |

## Export determinism targets

| Setting | TARGET | Notes |
|---|---|---|
| Tar format | POSIX (PAX) | Use PAX for portability. GNU extensions are not stable across crate versions |
| Zstd compression level | 3 | Default level. Must be pinned, not left to library default which may change |
| Tar mtime | 0 | Unix epoch. All entries normalized |
| Tar uid/gid | 0 | Root. Normalized to prevent machine-specific values |
| Tar username/groupname | (empty) | Omit to prevent machine-specific values |
| PAX extended headers | minimal | Only size and path. No atime, ctime, or OS-specific headers |

## Backpressure control-loop budgets

| Budget | TARGET | Notes |
|---|---:|---|
| Backpressure evaluation interval ms | 100 | Cadence for checking ladder transition conditions |
| Queue pressure raise ratio | 0.80 | Fraction of queue capacity that triggers escalation consideration. Applied to `queue_pressure` as defined in `docs/BACKPRESSURE_POLICY.md` (Ladder transition semantics v0.1) |
| Queue pressure clear ratio | 0.50 | Fraction of queue capacity below which recovery is considered. Applied to `queue_pressure` as defined in `docs/BACKPRESSURE_POLICY.md` (Ladder transition semantics v0.1) |
| De-escalation dwell ms | 2_000 | Stable-below-clear dwell before stepping down one ladder level. See `docs/BACKPRESSURE_POLICY.md` (Ladder transition semantics v0.1) |

## Write-path resilience budgets

| Budget | TARGET | Notes |
|---|---:|---|
| Append stall alarm ms | 250 | Append-path stall budget before alarm and reporting. See `docs/BACKPRESSURE_POLICY.md` failure mode `FM-APPEND-FAIL` |
| Blob fsync timeout ms | 1_000 | Blob durability operation budget before failure handling. See `docs/BACKPRESSURE_POLICY.md` failure mode `FM-BLOB-WRITE-FAIL` |
| EventLog max line bytes | 1_048_576 | Reject oversized serialized event lines to prevent unbounded memory |
| Safe-failure flush max ms | 5_000 | Upper bound on best-effort flush while entering safe failure posture. See `docs/BACKPRESSURE_POLICY.md` (Safe failure posture) |

<!-- DOCS_GUARD:END CAPACITY_TABLES -->
