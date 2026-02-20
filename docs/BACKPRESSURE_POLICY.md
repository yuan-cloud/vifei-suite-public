# Backpressure policy · v0.1

This document is constitutional.
It defines tiers, the degradation ladder, and the safe failure posture.

It is the single source of truth for backpressure semantics and projection honesty mechanics.

---

## Tiers

### Tier A (never drop, never reorder)

Tier A is lossless. It is always appended in the observed order, then assigned `commit_index` by the single append writer.

Rules:

- Never drop Tier A.
- Never reorder Tier A. Canonical order is `commit_index`.
- If Tier A cannot be appended, the system must alarm and enter safe failure posture.

Tier A includes the global core set defined in `PLANS.md` D2, plus importer-declared Tier A extensions.

### Tier B

Tier B may be sampled, aggregated, or collapsed under load.
Tier B must never cause Tier A loss.

### Tier C

Tier C is best-effort telemetry and can be dropped under stress.

---

## Degradation ladder

The ladder is the only allowed order of degradation.
Projections must report the current ladder level in the Truth HUD.

<!-- DOCS_GUARD:BEGIN LADDER_LEVELS -->

- **L0 Normal**. 1:1 events rendered.
- **L1 Aggregate**. Bin and summarize Tier B and Tier C. Tier A remains 1:1.
- **L2 Collapse**. Collapse Tier B and Tier C into counts and histograms. Tier A remains 1:1.
- **L3 Reduce Fidelity**. Reduce UI fidelity (fewer redraws, simplified rendering). Tier A remains 1:1.
- **L4 Freeze UI**. Freeze non-HUD projection panes. Continue ingesting Tier A. Truth HUD remains visible and shows freeze.
- **L5 Safe failure posture**. Stop ingest. Keep last known-good truth readable.

<!-- DOCS_GUARD:END LADDER_LEVELS -->

---

## Projection invariants v0.1 (honesty mechanics only)

These are narrow, auditable rules.

<!-- DOCS_GUARD:BEGIN PROJECTION_INVARIANTS -->

- Projections must never fabricate events.
- Projections must never reorder truth.
- All projections must iterate events by `commit_index`. Never by timestamp.
- Projections may summarize Tier B and Tier C per the ladder, but must confess what they did in the Truth HUD.
- Projections must visually distinguish synthesized fields (events with `synthesized: true`) from observed data in Forensic Lens.
- Truth HUD must confess at minimum: current ladder level, aggregation mode and bin size, queue pressure indicator, Tier A drops counter, export safety state, and `projection_invariants_version`.
- At `L4`, non-HUD panes may freeze, but Truth HUD confession fields remain live from ingest state.

<!-- DOCS_GUARD:END PROJECTION_INVARIANTS -->

### Versioning

The projection invariants version for v0.1 is the string `"projection-invariants-v0.1"`.

This version must change (by incrementing the version suffix) whenever:
- A projection invariant rule is added, removed, or modified in this section.
- The ViewModel include/exclude list for hashing changes.

The version string is embedded in ViewModel, `metrics.json`, and `timetravel.capture` to ensure hash stability is traceable to a specific set of invariant rules.

---

## Ladder transition semantics (v0.1)

Ladder transitions are deterministic and auditable.

<!-- DOCS_GUARD:BEGIN LADDER_TRANSITION_SEMANTICS -->

- Transition checks run on the cadence and pressure thresholds defined in `docs/CAPACITY_ENVELOPE.md` (see Backpressure control-loop budgets).
- The budget labels that matter for transitions are: Backpressure evaluation interval ms, Queue pressure raise ratio, Queue pressure clear ratio, and De-escalation dwell ms.
- `queue_pressure` is a normalized ratio in `[0.0, 1.0]`, computed as `queue_depth / queue_capacity` (clamped).
  - The same value is used for raise and clear comparisons and is exported as `metrics.json.queue_pressure`.
- Escalation moves one level at a time (`L0 → L1 → L2 → L3 → L4`), except fatal storage failures which transition directly to `L5`.
- Recovery moves one level at a time only after pressure remains below clear thresholds for the configured dwell window.
- Every transition emits a `PolicyDecision` Tier A event with at least `from_level`, `to_level`, `trigger`, and pressure context (including `queue_pressure`).
- `metrics.json.degradation_transitions` must preserve transition order exactly.
- No flapping: the dwell window prevents oscillation between adjacent levels.

<!-- DOCS_GUARD:END LADDER_TRANSITION_SEMANTICS -->

---

## Failure modes v0.1

<!-- DOCS_GUARD:BEGIN FAILURE_MODES -->

- **FM-APPEND-FAIL**. Append writer cannot durably append Tier A (for example fsync error, or exceeding the write-path stall alarm budget in `docs/CAPACITY_ENVELOPE.md`). Enter `L5` safe failure posture immediately.
- **FM-BLOB-WRITE-FAIL**. A payload that requires blob storage cannot be durably written (for example fsync error, or exceeding the blob fsync timeout budget in `docs/CAPACITY_ENVELOPE.md`). Emit `Error` Tier A event if possible, then enter `L5`.
- **FM-PROJECTION-OVERBUDGET**. Projection exceeds frame budgets while ingest remains healthy (see UI projection budgets in `docs/CAPACITY_ENVELOPE.md`). Follow ladder `L0` through `L4`. Do not enter `L5`.
- **FM-EXPORT-UNSAFE**. Secret detection during export. Refuse export and emit refusal report; do not change ingest ladder level.

<!-- DOCS_GUARD:END FAILURE_MODES -->

---

## Safe failure posture

If Tier A cannot be appended or the append writer detects an unrecoverable storage failure:

<!-- DOCS_GUARD:BEGIN SAFE_FAILURE_POSTURE -->

- Attempt to emit a clear `Error` Tier A event describing the failure cause.
  - If the same failure prevents appending the error event, emit an equivalent message to stderr and return a non-zero exit code.
  - Never claim success in this posture.

- Stop ingest immediately. In v0.1 local-only mode, this means the import or Tour command stops reading input and exits after flushing what it can.

- Keep the UI usable in read-only mode for investigation, using the last known-good on-disk EventLog and blobs.
  - The TUI must not attempt further writes while in safe failure posture.

- Best-effort flush while entering safe failure posture is bounded by the Safe-failure flush max ms budget in `docs/CAPACITY_ENVELOPE.md`.

- Provide a clear operator message describing how to recover, for example free disk space and rerun.

This is not graceful degradation. This is an alarm and a safe stop.

<!-- DOCS_GUARD:END SAFE_FAILURE_POSTURE -->
