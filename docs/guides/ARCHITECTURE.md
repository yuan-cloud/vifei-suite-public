# Architecture Guide

This is the first thing a new agent reads after `AGENTS.md`.

---

## System overview

Vifei Suite is a deterministic, local-first terminal cockpit for
recording, replaying, and safely sharing AI agent runs as evidence bundles.

**North star:** The EventLog is truth. The UI is a projection. Under
overload, truth never degrades — only the projection degrades.

---

## Crate layout

```text
vifei-suite/
  crates/
    vifei-core/       # EventLog, reducer, projection, ViewModel, blob store
      src/event.rs         # M1: ImportEvent, CommittedEvent, EventPayload, Tier
      src/eventlog.rs      # M2: append-only writer, sole commit_index assigner
      src/blob_store.rs    # M2: BLAKE3 content-addressed blob store
      src/reducer.rs       # M4: pure (State, Event) -> State
      src/projection.rs    # M5: deterministic State -> ViewModel
    vifei-import/     # Importers (Agent Cassette first)
      src/cassette.rs      # M3: Cassette JSONL -> ImportEvent
    vifei-export/     # M8: share-safe export + redaction
    vifei-tui/        # M6: Ratatui TUI shell, lenses, Truth HUD
    vifei-tour/       # M7: deterministic stress harness
  fixtures/                # Test fixtures (synthetic Agent Cassette sessions)
  docs/
    constitution/          # CAPACITY_ENVELOPE.md, BACKPRESSURE_POLICY.md
    guides/                # This directory
    RISK_REGISTER.md       # Append-only risk log
```

---

## Data flow: ingest to TUI

```text
Agent Cassette JSONL
  │
  ▼
cassette::parse_cassette()  →  Vec<ImportEvent>
  │                              (no commit_index — D6)
  ▼
EventLogWriter::append()    →  CommittedEvent (JSONL on disk)
  │                              (commit_index assigned here only)
  ├── BlobStore (if payload > 16 KiB inline threshold)
  ├── ClockSkewDetected (if backward timestamp > 50 ms)
  │
  ▼
reducer::replay()           →  State (BTreeMap-only, no HashMap)
  │                              checkpoint every 5000 events
  ▼
projection::project()       →  ViewModel (hashable, no terminal state)
  │                              viewmodel.hash = BLAKE3(...)
  ▼
TUI render                  →  Incident Lens / Forensic Lens / Truth HUD
```

**Key invariant:** Every arrow preserves determinism. Same input at each
stage always produces the same output. No wall clock, no RNG, no HashMap
in serialized paths.

---

## Truth taxonomy

| Category | Examples | Deletable? |
|----------|----------|------------|
| **Canonical truth** | EventLog JSONL, content-addressed blobs | Never |
| **Derived / rebuildable** | SQLite cache, checkpoints, ViewModel, Tour artifacts, export bundles | Yes — rebuild from truth |

See `PLANS.md` § "Truth Taxonomy" for the full definition.

---

## Two-type pattern (D6 enforcement)

Importers produce `ImportEvent` (no `commit_index` field). The append
writer converts to `CommittedEvent` via `CommittedEvent::commit()`. This
makes it a compile-time error for importers to set `commit_index`.

```text
ImportEvent  ──commit()──▶  CommittedEvent
  (importer)                  (append writer only)
```

---

## BLAKE3 content addressing (blake3 v1.8.3)

All content hashes in Vifei use BLAKE3 (256-bit / 64 hex chars).

### Blob store pattern

```rust
use blake3;

// One-shot hash for small inputs
let hash = blake3::hash(data);
let hex = hash.to_hex().to_string();  // lowercase, 64 chars

// Incremental hash for streaming / composite inputs
let mut hasher = blake3::Hasher::new();
hasher.update(b"prefix");
hasher.update(&serialized_bytes);
let hash = hasher.finalize().to_hex().to_string();
```

### Where BLAKE3 is used

| Hash | Input | Module |
|------|-------|--------|
| `payload_ref` | Raw blob bytes on disk | `blob_store.rs` |
| `state_hash` | `reducer_version` + canonical `State` JSON | `reducer.rs` |
| `viewmodel.hash` | `projection_invariants_version` + canonical `ViewModel` JSON | `projection.rs` (M5) |
| `bundle_hash` | Export bundle contents | `export/` (M8) |

### Blob store layout

```text
blobs/
  {first-2-hex}/
    {full-64-char-hex}
```

Two-char prefix reduces per-directory inode pressure. Deduplication is
natural: same content produces same hash, second write is a no-op.
Atomic writes via temp file + rename. Fsync for durability.

---

## Constitution docs

Behavioral contracts under load live in exactly two docs:

- `docs/CAPACITY_ENVELOPE.md` — numeric thresholds and targets
- `docs/BACKPRESSURE_POLICY.md` — tiers, degradation ladder, failure modes

**Never duplicate their content.** Link to them. See `AGENTS.md` § "Rule 3".

---

## Lessons from Anthropic agent team research

From the [C compiler experiment](https://www.anthropic.com/engineering/building-c-compiler)
(Feb 2026):

1. **Test harness quality > code quality.** The Tour stress harness (M7) is
   how we prove correctness — invest heavily in its fidelity.
2. **Strong types catch bugs.** The two-type pattern, `BTreeMap`-only State,
   and `Tier` enum leverage Rust's type system as a force multiplier.
3. **Specialization works.** Different agents can own different crates/beads
   in parallel (M2, M3, M4 after M1 completes).
4. **CI as gatekeeper.** Quality gates (`fmt`, `clippy`, `test`) run before
   every commit. New code must not break existing tests.
5. **Print concise output.** Tour artifacts and test output should be
   structured and parseable, not verbose prose.

---

## Quick reference: invariants

| ID | Name | One-liner |
|----|------|-----------|
| I1 | Forensic truth | EventLog is truth, Tier A is lossless |
| I2 | Deterministic projection | Same EventLog + invariants → same ViewModel |
| I3 | Share-safe export | Export refuses when secrets remain |
| I4 | Testable determinism | Hashes and replay prove stability in CI |
| I5 | Loud failure | Tier A write failure → alarm + safe stop |
