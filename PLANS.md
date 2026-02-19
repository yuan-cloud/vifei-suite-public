# PLANS.md · Vifei Suite · v0.1

> **How to read this doc**
> This is the single planning artifact for v0.1.
> Agents start here, then read `AGENTS.md` for behavioral rules.
> Target values and thresholds live only in the two constitution docs (see **v6.2 Constitution** below).
> This file links and summarizes. It must not duplicate constitution tables, ladder steps, or numeric thresholds.

---

## CURRENT (plain, 3 sentences)

We are building a terminal-first cockpit that records and replays AI agent runs as deterministic evidence bundles.
It stays truthful under load by treating the EventLog as the source of truth and the UI as a degradable projection that must never lie.
A built-in Tour stress harness emits repeatable artifacts so CI can adjudicate determinism, safety, and overload behavior.

## CURRENT (technical, 3 sentences)

Agent Cassette JSONL is imported into an append-only JSONL EventLog where a single writer assigns `commit_index` as the canonical replay order.
Large payloads are stored as content-addressed blobs, a rebuildable SQLite index powers fast investigation queries, and a pure reducer plus deterministic projection produces a stable `viewmodel.hash`.
Two contract docs, `docs/CAPACITY_ENVELOPE.md` and `docs/BACKPRESSURE_POLICY.md` (including "Projection invariants v0.1"), define overload semantics, and `vifei tour --stress` emits proof artifacts that CI asserts.

---

## v0.1 WEDGE (keep this glued to the repo)

Vifei Suite is a local-first, terminal-first flight recorder for AI agent runs that produces deterministic, replayable evidence bundles.
It is safe to share because export is redaction-first and refuses to produce an unsafe bundle.
Under overload, truth never degrades. Only the projection degrades, and Tour plus CI prove this with artifacts.

---

## TRUTH TAXONOMY (what is truth, what is derived)

This section exists to prevent drift.

### Canonical truth

These are the only things Vifei treats as forensic truth:

1. Append-only EventLog (JSONL).
2. Content-addressed blobs referenced by the EventLog.
3. Canonical replay order is `commit_index`, assigned by the single append writer.

Everything else is derived.

### Derived and rebuildable

These may be deleted and regenerated without changing truth:

- SQLite index cache.
- Reducer checkpoints.
- ViewModel, TUI rendering, and snapshots.
- Tour proof artifacts.
- Export bundles and refusal reports (derived from truth plus redaction policy).

Rule. If a feature needs to be "true", it must be in the EventLog. Not in SQLite, not in the UI, not in Tour output.

---

## v6.2 CONSTITUTION (two authoritative docs, and only two)

These are canonical. The rest of the repo must link to them and must not duplicate their numbers, tiers, ladder steps, or failure mode definitions.

- `docs/CAPACITY_ENVELOPE.md`. Target values only in `TARGET` columns (numeric thresholds and pinned constants). NORMAL vs STRESS. Everything is `TARGET` until Tour measures.
- `docs/BACKPRESSURE_POLICY.md`. Tiers, degradation ladder, failure modes. Includes a narrowly scoped "Projection invariants v0.1" section. Projection invariants are honesty mechanics only.

Rule. If a PR changes behavior under load, it must update one of these docs or add a test that enforces the doc.

### Constitution echo guard (required in v0.1)

v0.1 must include an automated docs guard that prevents accidental copy-paste drift:

- The constitution docs mark guarded snippets with HTML comments (for example `<!-- DOCS_GUARD:BEGIN ... -->` and `<!-- DOCS_GUARD:END ... -->`).
- A `docs_guard` test runs in CI and fails if any non-constitutional `*.md` file contains any line that is a character-exact match (after trimming leading/trailing whitespace) with any line inside a guarded snippet. The test ignores blank lines and lines that are only markdown formatting (e.g. `---`, `|---|---|`).
- This makes drift a failing test, not a review argument.

---

## LOCKED v0.1 DECISIONS (do not relitigate casually)

### D1. First importer

Agent Cassette JSONL importer is the first supported ingestion path.
We may implement an internal mapping layer to reuse later.
We do not ship a user-facing "generic mapping file" feature in v0.1.

### D2. Tier A minimal set (never drop, never reorder)

Global core Tier A events:

`RunStart`, `RunEnd`, `ToolCall`, `ToolResult`, `PolicyDecision`, `RedactionApplied`, `Error`, `ClockSkewDetected`.

Plus. Importer-declared Tier A extensions (source-specific).

Rule. Do not invent meaning. If any field is synthesized or inferred, mark it explicitly as `synthesized: true` and surface the marker in UI.

### D3. Runtime mode

v0.1 is local-only (CLI plus TUI). No long-running daemon.

Design note. Ingestion architecture should permit future multi-process inputs via an optional local socket or stdin stream, without refactoring the core invariants.

### D4. Storage baseline

Canonical truth. Append-only JSONL EventLog plus content-addressed blobs for large payloads.
Derived cache. SQLite index from day 1, explicitly rebuildable (for example `vifei reindex`) and never treated as truth.

### D5. UX target

Correctness target. Deep investigation. Entry behavior. Incident triage.

Default screen is Incident Lens. One key (`Tab`) toggles into Forensic Lens timeline plus inspector.

### D6. Canonical ordering model

- Canonical replay order is `commit_index`, assigned by the single append writer at ingest time.
- `timestamp_ns` is informative metadata only. Never used for canonical ordering.
- Clock skew is surfaced via Tier A event `ClockSkewDetected`. We do not silently merge clocks.

Implementation constraints:

- `commit_index` is assigned in exactly one place. The append writer.
- Importers must not sort by timestamp.
- Projections must iterate events by `commit_index`, never by timestamp.

Design note on `commit_index` ownership:

- Importers produce typed Event values WITHOUT a `commit_index`. The append writer is the sole assigner.
- In Rust, this means the importer-facing Event type should make `commit_index` clearly unset (e.g. `Option<u64>` that the append writer fills, or a separate `ImportEvent` struct that lacks the field entirely and is wrapped into a `CommittedEvent` by the append writer).
- The chosen approach must make it a compile-time or immediate-runtime error for an importer to set `commit_index`.

### D7. Branch policy

Open. Do not hardcode main versus master synchronization rules in v0.1.

---

## NON-NEGOTIABLE INVARIANTS (behavioral contracts)

I1. **Forensic truth.** EventLog is forensic truth. Tier A is lossless and ordered by `commit_index`.
I2. **Deterministic projection.** UI is a deterministic projection of EventLog plus projection invariants. Under overload, UI degrades before truth.
I3. **Share-safe export.** Export refuses when secrets remain, and emits a refusal report explaining exactly what blocked export.
I4. **Testable determinism.** Determinism is testable and enforced by CI using hashes and replay checks.
I5. **Loud failure.** If Tier A cannot be recorded, the system must alarm loudly and enter a defined safe failure posture. No silent limp mode.

---

## CORE CONCEPTS (short definitions)

- **EventLog**. Append-only record of what happened. Canonical truth.
- **Reducer**. Pure function `(State, Event) -> State` that rebuilds state from events.
- **Projection**. Deterministic function `State -> ViewModel`. No side effects.
- **ViewModel**. Hashable data structure that drives the TUI. Excludes terminal size, focus state, cursor blink.
- **Backpressure**. Overload policy that protects Tier A and degrades UI first. Defined in `docs/BACKPRESSURE_POLICY.md`.
- **Tour**. Deterministic stress harness that produces proof artifacts for CI.
- **Truth HUD**. Always-visible status strip that confesses system truthfulness state.
- **Synthesized**. A field or value not present in the original source data, inferred or invented by the importer or ingestion pipeline. Marked `synthesized: true` on the event. Downstream contracts:
  - Reducer: must process synthesized events identically to non-synthesized (they are still truth once committed). The `synthesized` flag is metadata, not a processing directive.
  - Projection: should surface synthesized markers in Forensic Lens so investigators can distinguish inferred from observed data. See `docs/BACKPRESSURE_POLICY.md` § "Projection invariants v0.1".
  - Export: synthesized fields are included in bundles. The `synthesized` marker is preserved so consumers can assess provenance.

---

## DETERMINISM CONTRACT (what must be stable)

Determinism is not vibes. It is bytes.

### Canonical replay

- Replay order is `commit_index` only.
- Any timeline view is a projection. It may group, collapse, or summarize events, but it may not reorder truth.

### Hash boundaries

- `state_hash`. Hash of reducer state after replay. Inputs are EventLog plus reducer version. Output must be stable.
  - Computation: BLAKE3 of the deterministically serialized reducer State struct. Serialization must use the same canonical rules as EventLog (stable field order, BTreeMap or sorted containers, no floats without quantization).
  - `reducer_version`: a string constant defined in the reducer module (e.g. `"reducer-v0.1"`). It is included as a prefix to the hash input so that reducer logic changes produce visibly different hashes.
- `viewmodel.hash`. Hash of ViewModel. Inputs are EventLog plus projection invariants version. Output must be stable.
- `bundle_hash`. Hash of the share-safe export bundle. Inputs are EventLog plus blobs plus redaction config version. Output must be stable.

### EventLog JSONL encoding (v0.1, required)

- One JSON object per line.
- Newline-terminated (`\n`), UTF-8 bytes.
- No pretty printing. No embedded newlines in a single event line.
- The append writer is the only component that writes JSONL. Importers emit typed Events. Reducer and UI do not write JSONL.

### Canonical serialization requirements

- Avoid `HashMap` in any structure that is hashed or serialized for determinism checks.
- If you need a map, use `BTreeMap` or a sorted `Vec<(K, V)>`.
- If you must store JSON objects with dynamic keys, either:
  1) store the raw bytes as a blob and address them, or
  2) canonicalize before hashing (sort keys, stable number formatting, stable string escaping).

### Floats policy (v0.1, pragmatic)

- Floats are allowed in derived artifacts (metrics, UI) only if formatting is explicitly controlled.
- Floats are discouraged in hashed truth surfaces. If unavoidable, quantize and document precision in code and tests.

### ID and reference contracts (clarify early to avoid schema thrash)

- `run_id` is the identity of a run. It scopes uniqueness.
- `event_id` must be unique within `run_id`. Recommended default if source has no ID: `"{source_id}:{source_seq}"`.
- `source_seq` must be monotonic per `source_id` for a given run, when available. If unknown, use best-effort and mark `synthesized: true`.

`payload_ref` contract:

- `payload_ref` is lowercase hex BLAKE3 digest of the blob bytes as stored on disk.
- The blob store path format is an implementation detail, but `payload_ref` is the stable identifier.

---

## TECH STACK (v0.1)

This section is descriptive, not constitutional.

| Layer | Choice | Notes |
|---|---|---|
| Language | Rust (stable) | All CLI, TUI, core logic |
| TUI framework | Ratatui | Snapshot-testable via `ftui-harness` |
| Serialization | `serde` plus `serde_json` | Deterministic output requires stable container ordering |
| Storage | Append-only JSONL plus content-addressed blobs | Canonical truth |
| Cache | SQLite (for example via `rusqlite`) | Rebuildable derived index. Never truth |
| Hashing | BLAKE3 | For `viewmodel.hash`, blob addressing, bundle integrity |
| CLI | `clap` | Subcommands: import, view, export, tour |
| Testing | `cargo test` plus Tour harness | CI enforced |

Deferred (explicitly not v0.1). Daemon mode, multi-process ingestion sequencer, third-party importers (OTLP), web companion.

---

## REQUIRED PROOF ARTIFACTS (Tour outputs)

Even if slow at first, the artifact shapes must exist early.

`vifei tour --stress` must emit:

| Artifact | Format | Purpose |
|---|---|---|
| `metrics.json` | JSON | Timing, throughput, drop counts, queue depths |
| `viewmodel.hash` | Plain text (BLAKE3 hex) | Determinism proof for same input, same output |
| `ansi.capture` | ANSI text or asciicast v2 | Visual regression baseline |
| `timetravel.capture` | JSON or JSONL | Deterministic time-travel replay artifact |

Minimum CI assertions for v0.1:

- Tier A drops equals 0
- Degradation transitions respect the ladder order defined in `docs/BACKPRESSURE_POLICY.md`
- Same EventLog plus same Projection invariants yields the same `viewmodel.hash`

### Artifact schema contracts (minimum required keys)

The exact values may evolve. These shapes must exist so CI can assert invariants.

`metrics.json` must include at minimum:

- `projection_invariants_version`
- `event_count_total`
- `tier_a_drops`
- `max_degradation_level`
- `degradation_level_final`
- `degradation_transitions` as an ordered list (each entry is an object that includes at minimum: `from_level`, `to_level`, `trigger`, `queue_pressure`)
- `aggregation_mode` and bin size
- `queue_pressure`
- `export_safety_state`

Notes:
- `queue_pressure` is a normalized ratio in `[0.0, 1.0]` that is used for ladder transitions. Its definition is canonical in `docs/BACKPRESSURE_POLICY.md` under "Ladder transition semantics v0.1".
- `max_degradation_level` and `degradation_level_final` should use the ladder level identifiers (e.g. `L0`, `L1`, ...), without re-listing the ladder steps here.

`timetravel.capture` must include at minimum:

- `projection_invariants_version`
- an ordered list of seek points, each with `commit_index`, `state_hash`, and `viewmodel.hash`

`refusal-report.json` must include at minimum:

- `report_version` (string, e.g. `"refusal-v0.1"`)
- `eventlog_path` (string, path to the source EventLog)
- `blocked_items` as an ordered list, each with:
  - `event_id`
  - `field_path` (dot-delimited path within the event, e.g. `"payload.api_key"`)
  - `matched_pattern` (the pattern name or regex that triggered the block)
  - `blob_ref` (optional, if the secret was found in a blob rather than inline)
- `scan_timestamp_utc` (ISO 8601, informational only)
- `scanner_version` (string)

`viewmodel.hash` must be a single line of lowercase hex, newline-terminated.

---

## TRUTH HUD (must always be visible)

A tiny always-visible strip that confesses system truthfulness state:

- Current degradation ladder level (as defined in `docs/BACKPRESSURE_POLICY.md`)
- Aggregation mode and bin size (for example `1:1`, `10:1`, `collapsed`)
- Backlog or queue pressure indicator
- Tier A drops counter (must be `0`)
- Export safety state. `UNKNOWN`, `CLEAN`, `DIRTY`, `REFUSED`
- Projection invariants version (also in Tour artifacts and `metrics.json`, not in every event row)

---

## HERO LOOP (v0.1 end-to-end slice)

Goal. One workflow that is boringly reliable under stress.

```text
1) vifei import cassette ./session.jsonl
2) vifei view ./eventlog.jsonl
3) [Tab] toggle. Incident Lens to Forensic Lens
4) vifei export --share-safe -o ./bundle.tar.zst ./eventlog.jsonl
5) vifei tour --stress ./fixtures/large-session.jsonl
```

---

## EXPECTED REPO LAYOUT (v0.1)

This is a guide, not a mandate. Agents may adjust boundaries if justified, but must not violate the Truth taxonomy.

```text
vifei-suite/
├── PLANS.md
├── AGENTS.md
├── Cargo.toml
├── docs/
│   ├── CAPACITY_ENVELOPE.md
│   ├── BACKPRESSURE_POLICY.md
│   └── RISK_REGISTER.md
├── crates/
│   ├── vifei-core/              # EventLog, reducer, projection, ViewModel
│   ├── vifei-import/            # Importers (Agent Cassette first)
│   ├── vifei-export/            # Share-safe export plus redaction
│   ├── vifei-tui/               # Ratatui shell, lenses, Truth HUD
│   └── vifei-tour/              # Tour stress harness
└── fixtures/                         # Test fixtures (Agent Cassette sessions)
```

---

## MILESTONES (bead-friendly)

Each milestone is a bead. A bead is a discrete unit of work that one agent can claim, execute, verify, and hand off.

Definition of done for any bead:

1. Acceptance criteria pass.
2. Quality gates pass, see `AGENTS.md`.
3. Risk assessment entry appended to `docs/RISK_REGISTER.md`.
4. Handoff note produced, see `AGENTS.md`.

### Dependency DAG

```text
M0 -> M1 -> M2 -> M3
            |
            v
            M4 -> M5 -> M6 -> M7
            |
            +---> M8
```

Reading the DAG: arrows point from dependency to dependent. M2 feeds M3, M4, and M8. M4 feeds M5. M5 feeds M6. M6 feeds M7.

---

### M0. Repo governance surface

| Field | Value |
|---|---|
| Depends on | Nothing. Bootstrap |
| Inputs | This document |
| Outputs | `PLANS.md`, `AGENTS.md`, constitution docs, risk register, workspace skeleton, docs_guard test |
| Files touched | Governance docs, workspace `Cargo.toml`, crate stubs, docs_guard test |
| Done when | Quality gates pass on an empty workspace |

Acceptance criteria:

- `PLANS.md` and `AGENTS.md` exist at repo root.
- `docs/CAPACITY_ENVELOPE.md` exists with `TARGET` values only, no measured numbers.
- `docs/BACKPRESSURE_POLICY.md` exists with tiers, degradation ladder, failure modes, and Projection invariants v0.1.
- `docs/RISK_REGISTER.md` exists with the header template.
- Workspace skeleton exists so later beads do not need to edit shared wiring files:
  - `Cargo.toml` declares the workspace and each crate.
  - Each crate has a stub `src/lib.rs` or `src/main.rs`, plus stub module files matching later bead outputs.
- `docs_guard` test exists and runs in CI.
- Quality gates pass.

---

### M1. Event schema v0.1

| Field | Value |
|---|---|
| Depends on | M0 |
| Inputs | D2 Tier A event list, D6 ordering model |
| Outputs | Event types plus deterministic serde rules |
| Files touched | `crates/vifei-core/src/event.rs` plus unit tests |
| Done when | Round-trip serialization is byte-stable for every Tier A event |

Acceptance criteria:

- Defines all D2 Tier A events.
- Event struct includes fields:
  - `run_id`, `event_id`, `source_id`, `source_seq`, `timestamp_ns`, `commit_index`, `tier`
  - `payload_ref` optional, `synthesized` bool default false
- `commit_index` must be represented in a way that prevents importers from setting it. Recommended: `Option<u64>` that importers leave as `None`, or a two-type pattern (ImportEvent vs CommittedEvent) where only the append writer produces the committed form. The chosen approach must be documented in module docs with rationale.
- Module docs explain how byte stability is achieved, including which containers are allowed in hashed or serialized paths.
- Round-trip test. Serialize then deserialize then re-serialize yields identical bytes for each Tier A event variant.
- Round-trip test must cover: all Tier A variants, events with `payload_ref` set, events with `synthesized: true`, events with `source_seq` absent (if the type allows it).
- Event JSON encoding is canonical:
  - one JSON object per line, newline-terminated
  - no pretty printing, no embedded newlines
  - UTF-8 bytes on disk
  - field order is struct declaration order (serde default), documented in the module as canonical
  - Avoid `serde_json::Value` in any on-disk EventLog line and any hashed truth surface (`state_hash`, `viewmodel.hash`).
    - If dynamic JSON is unavoidable, store bytes as a blob and address them via `payload_ref`.
    - If a dynamic-key map must be hashed, implement explicit canonicalization (sorted keys, stable number formatting) and lock it with tests.
    - Do not rely on insertion-order preservation as a determinism strategy.
- Tier enum supports `A`, `B`, `C` with `Display` and `FromStr`.
- Tier serializes to JSON as a string: `"A"`, `"B"`, `"C"`.
- Tier B and C event variants are not required in v0.1. The schema must support them via a generic or extensible variant (e.g. `Event::Generic { tier: Tier, event_type: String, ... }`) so that future milestones can add variants without schema-breaking changes.

---

### M2. Append writer v0.1

| Field | Value |
|---|---|
| Depends on | M1 |
| Inputs | Event schema types, capacity targets |
| Outputs | Single writer append, blob store wiring |
| Files touched | `crates/vifei-core/src/eventlog.rs`, `crates/vifei-core/src/blob_store.rs` plus tests |
| Done when | Monotonic `commit_index` is enforced and tested. Clock skew detection tested |

Acceptance criteria:

- Single writer appends events to JSONL with monotonically increasing `commit_index`.
- For a new EventLog file, `commit_index` starts at 0 and increments by exactly 1 for each appended event.
- `commit_index` is assigned in exactly one code path, the append writer. Importers must not set or modify it.
- Content-addressed blob store:
  - Payloads above the inline threshold in `docs/CAPACITY_ENVELOPE.md` are stored as blob files.
  - `payload_ref` is the lowercase hex BLAKE3 digest of the blob bytes as stored on disk.
- `ClockSkewDetected` emitted when a source's `timestamp_ns` moves backward beyond tolerance in `docs/CAPACITY_ENVELOPE.md`.
- Tests:
  - Append 1000 events, verify `commit_index` is `0..999` with no gaps.
  - Write one event above the inline payload threshold, verify a blob is created and `payload_ref` resolves.
  - Inject backward timestamp, verify `ClockSkewDetected` is emitted.

---

### M3. Agent Cassette importer v0.1

| Field | Value |
|---|---|
| Depends on | M2 |
| Inputs | Append writer API, at least one Agent Cassette session fixture |
| Outputs | Agent Cassette JSONL importer |
| Files touched | `crates/vifei-import/src/cassette.rs` plus fixtures and tests |
| Done when | Imports a real fixture without reordering. Marks synthesized fields |

Acceptance criteria:

- Reads Agent Cassette JSONL and maps events to Vifei event schema.
- Does not sort by timestamp. Preserves source order exactly as received. Append writer assigns `commit_index`. The importer must never re-sort, deduplicate, or "fix history" based on timestamps — if a source has out-of-order timestamps, that is surfaced via `ClockSkewDetected`, not corrected.
- Any inferred or synthesized fields set `synthesized: true`.
- At least one real session file in `fixtures/` with README documenting provenance and redaction status.
- Test. Import fixture then read back EventLog. Verify Tier A present and `commit_index` monotonic.

---

### M4. Reducer plus checkpoints v0.1

| Field | Value |
|---|---|
| Depends on | M2 |
| Inputs | EventLog with test data, capacity targets |
| Outputs | Pure reducer, checkpoint support |
| Files touched | `crates/vifei-core/src/reducer.rs` plus tests |
| Done when | Determinism test passes. Checkpoint replay matches full replay |

Acceptance criteria:

- Pure function `fn reduce(state: &State, event: &Event) -> State`. No IO, no randomness, no wall clock reads.
- Checkpoint every N events (N is a TARGET in `docs/CAPACITY_ENVELOPE.md`). Checkpoint format is versioned.
- Checkpoint format includes: `reducer_version` string, `commit_index` of the last event reduced, and the serialized State.
- Rebuild from checkpoint plus replay equals full replay.
- `state_hash` computation: BLAKE3 of `reducer_version` concatenated with deterministically serialized State (see "Hash boundaries" in this doc). Documented near the hashing code with explicit include/exclude list.
- Determinism test: Same EventLog yields same `state_hash` across 10 repeated runs.

---

### M5. Projection plus viewmodel.hash v0.1

| Field | Value |
|---|---|
| Depends on | M4 |
| Inputs | Reducer state, projection invariants from `docs/BACKPRESSURE_POLICY.md` |
| Outputs | Deterministic projection and ViewModel types |
| Files touched | `crates/vifei-core/src/projection.rs` plus tests |
| Done when | `viewmodel.hash` is stable across repeated runs for same inputs |

Acceptance criteria:

- Deterministic function `fn project(state: &State, invariants: &ProjectionInvariants) -> ViewModel`.
- ViewModel excludes terminal size, focus state, cursor blink, wall clock, randomness.
- ViewModel includes enough confession state to make honesty auditable:
  - Tier A summaries
  - aggregation mode and bin size
  - current degradation ladder level
  - backlog or queue pressure indicator
  - Tier A drops counter
  - export safety state (`UNKNOWN` before M8 export scan lands; then `CLEAN`, `DIRTY`, or `REFUSED`)
  - `projection_invariants_version`
- `viewmodel.hash`. BLAKE3 of deterministically serialized ViewModel. Include and exclude list is explicit near the hashing code.
- Stability test. Same EventLog plus same invariants yields same `viewmodel.hash` across 10 runs.

---

### M6. TUI shell plus Incident Lens v0.1

| Field | Value |
|---|---|
| Depends on | M5 |
| Inputs | ViewModel, Ratatui |
| Outputs | TUI shell, Incident Lens, Forensic Lens stub, Truth HUD |
| Files touched | `crates/vifei-tui/src/` plus `src/main.rs` |
| Done when | TUI opens on an EventLog and renders Incident Lens. Truth HUD is auditable |

Acceptance criteria:

- TUI loads EventLog and displays Incident Lens as default. Run summary plus top anomalies.
- `Tab` toggles to Forensic Lens. Timeline scrubber plus event inspector for ToolCall, ToolResult, policy, and redaction.
- Truth HUD always visible, in both lenses, and it must render at minimum:
  - current degradation ladder level
  - aggregation mode and bin size
  - backlog or queue pressure indicator
  - Tier A drops counter
  - export safety state (`UNKNOWN` until M8)
  - projection invariants version
- Snapshot or capture test exists that makes Truth HUD presence auditable for a fixture EventLog.
- Clean exit with `q` or `Ctrl-C`.

---

### M7. Tour stress harness v0.1

| Field | Value |
|---|---|
| Depends on | M5 and M6 |
| Inputs | Projection, ViewModel, stress fixture(s), capacity targets |
| Outputs | Tour harness and 4 proof artifacts |
| Files touched | `crates/vifei-tour/src/` plus fixtures and CI assertions |
| Done when | `vifei tour --stress` emits all artifacts and CI asserts invariants |

Acceptance criteria:

- `vifei tour --stress` ingests a large fixture, runs full pipeline under simulated load, emits to `tour-output/`:
  - `metrics.json`
  - `viewmodel.hash`
  - `ansi.capture`
  - `timetravel.capture`
- `metrics.json` includes the required keys described in **Artifact schema contracts** above.
- CI assertions:
  - `tier_a_drops == 0`
  - degradation transitions are recorded and respect the ladder order
  - `viewmodel.hash` matches on re-run with same inputs
  - if any ladder transition occurs, there is a corresponding Tier A `PolicyDecision` event in the EventLog that encodes the same `from_level` and `to_level`, and `metrics.json.degradation_transitions` is derivable from those events
- Stress fixture in `fixtures/` meets the Large fixture events target in `docs/CAPACITY_ENVELOPE.md`.

---

### M8. Share-safe export v0.1

| Field | Value |
|---|---|
| Depends on | M2 |
| Inputs | EventLog, blobs, redaction rules |
| Outputs | Deterministic bundle, refusal report |
| Files touched | `crates/vifei-export/src/` plus tests |
| Done when | Bundle hash stable for clean logs. Refusal path tested for secret-seeded logs |

Acceptance criteria:

- `vifei export --share-safe -o bundle.tar.zst ./eventlog.jsonl` produces a deterministic archive. Same inputs yield the same `bundle_hash`.
- Deterministic bundling rules are explicit and tested:
  - archive entry list is sorted deterministically
  - archive metadata is normalized (for example mtime, uid, gid)
  - tar format, compression parameters, and header normalization are pinned to constants defined in `docs/CAPACITY_ENVELOPE.md` (see Export determinism targets)
  - refusal report and manifest entries are stably sorted
- Secret scanner checks event payloads and blob contents.
- If secrets detected. Export refuses and emits `refusal-report.json` listing every blocked item with event id, field path, and matched pattern. Schema is defined in **Artifact schema contracts** above.
- If clean. Export produces bundle with embedded integrity manifest.
- Tests:
  - Export clean fixture. Verify bundle hash stability.
  - Export secret-seeded fixture. Verify refusal and report contents.
  - Re-export same fixture on a second run. Verify `bundle_hash` matches.

---

## RISK ASSESSMENT LOOP (run between every bead)

After completing each bead, append findings to `docs/RISK_REGISTER.md`.

The Risk Register is not optional. It is how we prevent six-month drift.

---

## DEFERRED (explicitly not v0.1)

- Daemon mode or multi-process ingestion sequencer.
- OTLP or other third-party importers.
- Any web companion code.
- A third constitution doc.
- GHOSTLIGHT as a full product fork. Allowed only as fixture-driven vignette after the hero loop is proven.
- `security_meta` structured annotation on events. v0.1 secret scanning operates on payload content, not metadata annotations. Future versions may add a `SecurityAnnotation` struct to events for richer provenance tracking.
- Per-event digital signatures or Merkle chaining for tamper evidence. v0.1 relies on append-only JSONL and `commit_index` monotonicity.
