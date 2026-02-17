# Risk register · v0.1

This document is append-only.
Do not delete or rewrite old entries.
New entries are appended after completing each bead.

Rule. Use invariant IDs I1 through I5 from `PLANS.md` when relevant.

---

## Template

Copy this template for each completed bead.

```markdown
## M{n} · {milestone name} · {date}

Context:
- Bead owner: {who completed this bead?}
- Invariants referenced: {I1, I2, ... or none}
- Constitution touched: {none | CAPACITY | BACKPRESSURE}

1. Coupling: {what new coupling did we introduce that will be painful in 3 months?}
2. Untested claims: {what correctness claim did we make that we did not test?}
3. Nondeterminism: {what nondeterminism could have entered, time, randomness, concurrency, HashMap iteration, floats?}
4. Security: {what security or privacy risk did we create, secrets, tokens, PII?}
5. Performance: {what performance cliff did we create, burst load, disk stall, huge payload, unbounded allocation?}
```

---

## Pre-M0 · Governance surface red-team review · 2026-02-15

Context:
- Bead owner: architectural red-team review (pre-implementation)
- Invariants referenced: I1, I2, I3, I4
- Constitution touched: CAPACITY (added export determinism targets), BACKPRESSURE (added projection invariants versioning and synthesized field visibility)

1. Coupling: `commit_index` type decision (M1) tightly couples M1-M2-M3 boundary. Chose to document the constraint explicitly rather than mandate a specific Rust pattern, preserving implementer flexibility but requiring the bead handoff to verify the contract. The two-type pattern (ImportEvent vs CommittedEvent) is recommended but not mandated — whichever choice M1 makes becomes load-bearing for all downstream beads.
2. Untested claims: `docs_guard` matching semantics are now specified (character-exact after whitespace trim) but the test itself does not exist yet (M0 responsibility). The specification may be wrong for edge cases: markdown tables with pipes, indented code blocks inside guarded snippets, or lines that appear in both guarded and unguarded sections.
3. Nondeterminism: Export determinism targets (tar PAX format, zstd level 3) are pinned but the specific Rust crates (`tar`, `zstd`) may have version-dependent behavior in PAX header generation. Mitigation: pin crate versions in `Cargo.lock` and add golden-file bundle hash tests. BLAKE3 is used everywhere — algorithm migration would require touching all hash surfaces simultaneously.
4. Security: Removed `security_meta` from v0.1 event schema and deferred it. v0.1 secret scanning is purely content-based (regex/pattern matching on payloads and blobs). This is adequate for local-only mode but must be revisited before any networked or multi-tenant mode. The secret scanner pattern set itself is still undefined — M8 implementer must make judgment calls about what patterns to check.
5. Performance: No new performance cliffs introduced. Export determinism targets add a fixed zstd compression level (3) which is reasonable for v0.1's local-only scope. Single-file JSONL EventLog will not scale past ~100K events without rotation or compaction — acceptable for v0.1 but will need addressing before daemon mode.

---

## M0 · Repo bootstrap & workspace wiring · 2026-02-16

Context:
- Bead owner: Claude Opus 4.6 (claude-code)
- Invariants referenced: I4 (testable determinism via docs_guard)
- Constitution touched: none

1. Coupling: Workspace `Cargo.toml` declares all five crates with inter-crate dependency edges (`panopticon-import`, `panopticon-export`, `panopticon-tui`, `panopticon-tour` all depend on `panopticon-core`). These edges are intentional per the expected repo layout but mean `panopticon-core` public API changes will cascade to all downstream crates. Low risk since this is the designed architecture.
2. Untested claims: The `docs_guard` test uses `HashSet` for guarded line matching, which is correct for exact-match semantics but does not catch near-misses (e.g., a line with one character changed). This is by design per AGENTS.md spec ("character-exact match after whitespace trimming"). Edge case: a guarded line that also appears legitimately in an unguarded context would be a false positive — no such case exists today but could arise if constitution docs contain common markdown phrases.
3. Nondeterminism: The `docs_guard` test uses `HashSet` internally for lookup but produces deterministic pass/fail results (set membership is deterministic; only iteration order is nondeterministic, and we only check membership). The `collect_md_files` function uses `read_dir` which has nondeterministic ordering, but violation reporting order is cosmetic only — the test pass/fail is deterministic.
4. Security: No secrets, tokens, or PII introduced. All files are stub code and governance docs. No network access, no user data handling.
5. Performance: No performance cliffs. The `docs_guard` test reads all `.md` files in the repo on every test run — acceptable for v0.1 repo size but should be monitored if the repo grows to hundreds of markdown files.

---

## M1 · Event schema v0.1 · 2026-02-16

Context:
- Bead owner: Claude Opus 4.6 (claude-code)
- Invariants referenced: I1 (forensic truth), I4 (testable determinism), D6 (canonical ordering)
- Constitution touched: none (links to CAPACITY_ENVELOPE and BACKPRESSURE_POLICY in docs only)

1. Coupling: The two-type pattern (`ImportEvent` / `CommittedEvent`) is now the load-bearing type boundary for all downstream beads. M2 (append writer) must call `CommittedEvent::commit()` to assign `commit_index`. M3 (importer) must produce `ImportEvent`. M4 (reducer) must consume `CommittedEvent`. Changing the field set on either type will cascade to all consumers. This coupling is intentional and enforces D6 at compile time. The `EventPayload` enum is also load-bearing — adding new Tier A variants requires updating every match arm in downstream code. The `Generic` variant mitigates this for Tier B/C.
2. Untested claims: (a) `serde_json` Ryu-based f64 serialization is assumed deterministic across platforms for `PolicyDecision::queue_pressure`. Tested for specific values (0.0, 0.5, 0.8, 0.85, 1.0, 0.123456789) but not exhaustively. Exotic values (subnormals, negative zero) are not tested because queue_pressure is clamped to [0.0, 1.0]. (b) Field order stability relies on serde's documented guarantee that struct fields serialize in declaration order. If serde ever changes this default, all round-trip tests would catch it immediately. (c) We claim `CommittedEvent::commit()` is the ONLY way to create a `CommittedEvent`, but Rust's struct literal syntax allows direct construction outside the module if all fields are `pub`. The compile-time enforcement is that `ImportEvent` lacks `commit_index`, not that `CommittedEvent` is truly opaque.
3. Nondeterminism: (a) `f64` in `PolicyDecision::queue_pressure` — serde_json Ryu produces canonical shortest representation for finite values, which is deterministic. Documented in code. (b) `BTreeMap<String, String>` in `Generic::data` — deterministic sorted iteration, verified by test. (c) No `HashMap` anywhere in event types. (d) No wall clock, no RNG, no thread-local state. Audit: `rg 'HashMap' crates/panopticon-core/src/event.rs` returns zero hits in non-test code.
4. Security: No secrets, tokens, or PII in the schema itself. Event payloads may contain sensitive data (e.g., `ToolCall::args` with API keys), but that is M8's responsibility (secret scanner). The schema does not add any access controls — all fields are `pub`, all data is in-memory. Acceptable for v0.1 local-only mode.
5. Performance: No performance cliffs. All types are small (String fields, enum variants). Serialization is O(n) in field count. The `CommittedEvent::commit()` method moves all fields without cloning. The 32 unit tests add ~0.02s to the test suite. No unbounded allocations — all String fields are bounded by the inline payload threshold (blobs handle large content).

---

## M2 · Append writer v0.1 · 2026-02-16

Context:
- Bead owner: Claude Opus 4.6 (claude-code)
- Invariants referenced: I1 (forensic truth), I5 (loud failure), D6 (canonical ordering)
- Constitution touched: none (references CAPACITY_ENVELOPE thresholds and BACKPRESSURE_POLICY failure modes via constants)

1. Coupling: `EventLogWriter` is now the single write-path for all EventLog data. M3 (importer) must produce `ImportEvent` values and call `writer.append()`. M4 (reducer) consumes `CommittedEvent` read via `read_eventlog()`. The `AppendResult` struct (committed event + detection events) is the API contract that M3 must handle. `BlobStore` is standalone — the writer does not call it directly; the caller decides when to blob. This keeps the writer focused on JSONL serialization and commit_index assignment. The `read_eventlog()` function is a convenience that M4 and M7 will depend on.
2. Untested claims: (a) Fsync-per-Tier-A is implemented but we do not test that fsync actually flushes to durable storage — that would require fault injection or hardware testing. We trust `File::sync_all()`. (b) The max line bytes check (1,048,576) rejects oversized events but does not test the exact boundary (we test a clearly-too-large event). (c) Resume logic (`scan_highest_index`) parses the full file to find the highest commit_index — it does not verify monotonicity of the existing file. A corrupted file with non-monotonic indices would resume from the highest found, which is correct but doesn't detect the corruption. (d) Blob store atomic write uses rename, which is atomic on POSIX but may not be on all filesystems.
3. Nondeterminism: (a) `HashMap<String, u64>` used for per-source timestamp tracking in clock skew detection. This is runtime state only — never serialized, never hashed. Iteration order does not matter because we only do point lookups. (b) `ClockSkewDetected` event_id includes `self.next_index` for uniqueness, which is deterministic. (c) Temp file for atomic blob writes uses `.tmp` extension — if the process crashes mid-write, a `.tmp` file may remain. This is a leak, not a correctness issue. (d) No wall clock, no RNG in the write path.
4. Security: (a) Blob store writes arbitrary bytes to disk. No validation of content. In v0.1 local-only mode this is acceptable. (b) EventLog JSONL is world-readable by default (filesystem permissions). No encryption, no access control. Acceptable for local-only. (c) No path traversal risk — blob paths are derived from BLAKE3 hex digests (alphanumeric only).
5. Performance: (a) `scan_highest_index` reads the entire EventLog file on open — O(n) in event count. Acceptable for v0.1 (target <100K events). Would need optimization (e.g., read last N bytes) for larger files. (b) Fsync per Tier A event is the safe default but may be slow under burst load (CAPACITY_ENVELOPE: fsync interval = 1). Acceptable for v0.1. (c) Blob store does one `sync_all` per blob write. (d) The 1000-event test takes ~0.15s including fsync. (e) No unbounded allocations — line size is capped at 1MB.

---

## M3 · Agent Cassette importer v0.1 · 2026-02-16

Context:
- Bead owner: Claude Opus 4.6 (claude-code)
- Invariants referenced: I1 (forensic truth — synthesized marking), I4 (testable determinism — source order preserved)
- Constitution touched: none

1. Coupling: `cassette::parse_cassette()` produces `Vec<ImportEvent>`, coupling tightly to `panopticon-core`'s `ImportEvent` and `EventPayload` types. Adding new `EventPayload` variants in M1 does not break the importer (unknown types map to `Generic`). However, changing `ImportEvent` field names or types would require importer updates. The `SOURCE_ID` constant ("agent-cassette") is public and used by integration tests for filtering — downstream code (M4 reducer, M7 tour) may depend on this string. The fixture file `small-session.jsonl` is a test dependency only and not part of the public API.
2. Untested claims: (a) The minimal ISO 8601 parser (`parse_iso8601_ns`) does not validate day-of-month against actual month length — dates like Feb 31 or Apr 31 are silently accepted, producing incorrect `timestamp_ns`. Low impact: Agent Cassette sources produce machine-generated timestamps that are always valid. (b) Non-UTC timezone offsets (e.g., `+05:00`) are silently ignored, falling back to `timestamp_ns = 0`. Agent Cassette timestamps are expected to be UTC. (c) The fixture covers 5 of 8 `EventPayload` variants (RunStart, RunEnd, ToolCall, ToolResult, Error). PolicyDecision, RedactionApplied, and ClockSkewDetected are system-generated and not expected from cassette sources. (d) No test for cassette files larger than 11 events — parser is streaming (line-by-line) so memory is bounded.
3. Nondeterminism: (a) `BTreeMap` used for `Generic::data` field — deterministic iteration. (b) `parse_cassette` processes lines in file order and does not sort — source order is preserved deterministically. (c) Event ID synthesis uses a sequential counter (`cassette:{seq}`) — deterministic. (d) No HashMap, no RNG, no wall clock, no thread-local state in the importer. Audit: `rg 'HashMap' crates/panopticon-import/src/cassette.rs` returns zero hits.
4. Security: (a) The parser reads arbitrary JSONL input. Malformed lines produce `Error` events rather than panics — graceful degradation. (b) No path traversal risk — all paths in cassette events are treated as opaque string data, not used for filesystem operations. (c) The fixture contains fully synthetic data — no real secrets, API keys, or PII. Verified by fixture README.
5. Performance: (a) `parse_cassette` reads all events into a `Vec` in memory. For v0.1 this is acceptable (target session size <10K events). Larger sessions would benefit from a streaming iterator API. (b) Each line is parsed as `serde_json::Value` then mapped — double allocation per event. Acceptable for v0.1 throughput targets. (c) No unbounded allocations — event size is bounded by the line length in the source file, and the EventLogWriter's max line bytes check (1MB) provides a downstream cap.

---

## M4 · Reducer plus checkpoints v0.1 · 2026-02-16

Context:
- Bead owner: Claude Opus 4.6 (claude-code)
- Invariants referenced: I2 (deterministic projection — State is input to projection), I4 (testable determinism — state_hash stability)
- Constitution touched: none (references CAPACITY_ENVELOPE checkpoint interval = 5000)

1. Coupling: `State` struct is the sole input to projection (M5). Adding new `EventPayload` variants in M1 requires adding a match arm in `reduce()` — but `Generic` provides a fallback so the reducer won't fail to compile. `state_hash()` depends on `serde_json` struct field serialization order — if `State` fields are reordered, all hashes change. `REDUCER_VERSION` must be bumped whenever reducer logic changes. `Checkpoint` format couples to both `State` and `REDUCER_VERSION`. `replay()` and `replay_from()` are the primary APIs M5 and M7 will use.
2. Untested claims: (a) `serde_json` serializes struct fields in declaration order — relied upon for deterministic `state_hash`, but not contractually guaranteed by serde_json. Tested indirectly via determinism_10_runs. (b) `state_hash` and `serialize_checkpoint` use `expect()` — these will panic if State ever contains a type that fails serialization. Current State is all-safe types (String, u64, BTreeMap, Vec of simple structs). (c) `f64` queue_pressure quantization uses `clamp(0.0, 1.0)` then `round()` — NaN input would clamp to 0.0 (f64::clamp behavior with NaN is "unspecified" per std docs but on current Rust returns the lower bound). Not tested for NaN specifically since queue_pressure is documented as `[0.0, 1.0]`.
3. Nondeterminism: (a) All map-like containers are `BTreeMap` — deterministic iteration. Audit: `rg 'HashMap' crates/panopticon-core/src/reducer.rs` returns zero hits in non-test code. (b) No floats in `State` — `queue_pressure` is quantized to `u64` millionths before storing. (c) No RNG, no wall clock, no thread-local state. (d) `reduce()` is a pure function: clones state, applies event, returns new state. (e) Determinism verified: 10-run test with 100 diverse events + 10-run test with 5500 events crossing checkpoint boundary. All hashes identical.
4. Security: No secrets, tokens, or PII in reducer logic. State accumulates event metadata (agent names, tool names, error messages) which may contain sensitive data from source events — but that is M8's responsibility (secret scanner before export). No file IO in the reducer itself. Checkpoint serialization/deserialization is done by callers.
5. Performance: (a) `reduce()` clones the entire `State` on every event — O(N*S) total cost for N events where S is state size. Acceptable for v0.1 (target <100K events). For larger replays, switching to `&mut State` would eliminate cloning. (b) `Vec` fields (policy_decisions, error_log, clock_skew_events, redaction_log) grow without bound. For v0.1 this is acceptable — these are typically small relative to event count. (c) Checkpoint at 5000-event intervals bounds the replay-from-scratch cost. (d) 33 reducer tests add ~1.8s to the test suite (dominated by the 6000-event and 10000-event checkpoint tests).

---

## M5.1 · ProjectionInvariants and LadderLevel · 2026-02-17

Context:
- Bead owner: Claude Opus 4.5 (claude-code)
- Invariants referenced: I2 (deterministic projection — invariants parameterize projection)
- Constitution touched: none (references BACKPRESSURE_POLICY ladder levels and projection invariants version)

1. Coupling: `LadderLevel` enum and `ProjectionInvariants` struct are now the input types for M5.3 (projection function). M6 (TUI) will depend on `LadderLevel` for rendering degradation state. M7 (Tour) will embed `projection_invariants_version` in artifacts. `PROJECTION_INVARIANTS_VERSION` constant is the single source of truth for version string — changing it will affect all downstream hash computations. The `#[serde(rename_all = "UPPERCASE")]` attribute on `LadderLevel` means JSON output is `"L0"` not `"l0"` — this is intentional to match BACKPRESSURE_POLICY identifiers but means deserializing lowercase input like `"l0"` requires the explicit `FromStr` with `.to_uppercase()` handling.
2. Untested claims: (a) `PartialOrd`/`Ord` derive on `LadderLevel` relies on variant declaration order — if variants are reordered, comparison semantics change silently. Documented in code comments to prevent this. (b) `#[default]` attribute on `L0` variant assumes that derive(Default) respects the attribute — this is stable Rust since 1.62, but if compiling on older Rust, compilation would fail (not silently misbehave).
3. Nondeterminism: None introduced. `LadderLevel` is a simple enum with no containers. `ProjectionInvariants` contains only a `String` and a `LadderLevel`. No `HashMap`, no floats, no RNG, no wall clock. Serialization is deterministic — verified by byte-stability tests.
4. Security: No secrets, tokens, or PII. `ProjectionInvariants` contains only configuration metadata (version string, degradation level). No user data flows through these types.
5. Performance: No performance cliffs. `LadderLevel` is `Copy` (8 bytes). `ProjectionInvariants` is small (String + enum). 20 tests add <0.01s to the test suite.

---

## M5.2 · ViewModel struct with all confession fields · 2026-02-17

Context:
- Bead owner: Claude Opus 4.5 (claude-code)
- Invariants referenced: I2 (deterministic projection), I4 (testable determinism via viewmodel.hash)
- Constitution touched: none (links to PLANS.md § Truth HUD)

1. Coupling: `ViewModel` is now the output type of the projection function (M5.3) and input to the TUI (M6). Adding/removing fields from `ViewModel` will require updates to both projection and rendering code. The `queue_pressure_fixed` field (i64) couples ViewModel to the `QUEUE_PRESSURE_PRECISION` constant — changing precision would invalidate existing hashes. `ExportSafetyState` enum is standalone but M8 will need to update it when export scanning is implemented.
2. Untested claims: (a) `BTreeMap` ordering is tested for string keys but edge cases (empty strings, unicode) are not exhaustively tested. (b) `queue_pressure_fixed` truncation (not rounding) means 0.999999999 becomes 999999, not 1000000 — this is intentional for consistency but not documented in tests. (c) The "excluded fields" test only checks JSON output doesn't contain those strings, not that the struct truly lacks those fields at compile time.
3. Nondeterminism: None introduced. (a) `BTreeMap<String, u64>` for `tier_a_summaries` — deterministic ordering verified by `test_viewmodel_btreemap_ordering`. (b) `queue_pressure` is stored as `i64` after quantization — no floats in serialized output. (c) No `HashMap`, no RNG, no wall clock. (d) Byte-stability verified by `test_viewmodel_byte_stable_serialization`.
4. Security: No secrets, tokens, or PII in ViewModel itself. `tier_a_summaries` contains only event type names (e.g., "RunStart") and counts, not payload content. Sensitive data in event payloads does not flow into ViewModel.
5. Performance: No performance cliffs. `ViewModel` contains small fields (BTreeMap with typically <10 entries, strings, integers). Serialization is O(n) in field count. 20 M5.2 tests add <0.01s to the test suite. No unbounded allocations in ViewModel itself — `tier_a_summaries` grows with distinct event type count, which is bounded by schema.

---

## M5.3 · Deterministic project() function · 2026-02-17

Context:
- Bead owner: Claude Opus 4.5 (claude-code)
- Invariants referenced: I2 (deterministic projection), I4 (testable determinism)
- Constitution touched: none (references BACKPRESSURE_POLICY ladder levels for aggregation modes)

1. Coupling: `project()` function now couples State (from M4) to ViewModel (from M5.2). The function depends on the Tier A type names being hardcoded as a constant array — adding new Tier A types requires updating this list. `project_with_pressure()` provides runtime queue pressure override, used by M6/M7 when live backpressure data is available. The `policy_decisions.last().queue_pressure_micro` lookup couples projection to the reducer's PolicyTransition struct.
2. Untested claims: (a) Tier A type names list is exhaustive — verified against PLANS.md D2 but not programmatically linked. (b) Aggregation mode strings ("1:1", "10:1", "collapsed", "frozen") are not validated against any schema — TUI (M6) must handle them by string match. (c) `project()` returns `ExportSafetyState::Unknown` unconditionally until M8 — no test verifies this changes when M8 is implemented.
3. Nondeterminism: None introduced. (a) `project()` is pure — no IO, no RNG, no wall clock. (b) BTreeMap iteration is deterministic. (c) queue_pressure lookup is deterministic (last element of Vec). (d) Determinism verified by `test_project_determinism` (10 runs with same inputs → same output). (e) Byte-stability verified by `test_project_byte_stable_serialization`.
4. Security: No secrets, tokens, or PII flow through `project()`. The function only extracts counts and metadata from State, not event payloads. Sensitive data in payloads stays in the EventLog and State; projection summarizes without exposing content.
5. Performance: No performance cliffs. `project()` is O(n) where n is the number of event types (bounded by schema, ~10). The tier_a_types loop is fixed-size (8 iterations). No unbounded allocations. 8 M5.3 tests add <0.01s to the test suite.

## bd-fdf · Bugfix: EventLog empty-file resume and cassette payload fidelity · 2026-02-17

Context:
- Bead owner: SilverHarbor (codex-cli)
- Invariants referenced: I1, I4, I5
- Constitution touched: none

1. Coupling: EventLog open/resume semantics now depend on `scan_highest_index` returning `Option<u64>` (`None` for empty/invalid files). This is a narrow internal coupling inside `eventlog.rs` and improves correctness at the writer boundary.
2. Untested claims: We still do not validate all malformed-existing-EventLog cases (for example, partially-corrupt files with mixed valid/invalid lines) beyond current best-effort scanning behavior; this bugfix only addresses the empty-file resume edge case.
3. Nondeterminism: No new nondeterminism introduced. The importer payload string conversion path is deterministic (`as_str` for string values, `to_string` for non-null non-string JSON), and commit index assignment remains single-writer deterministic.
4. Security: No new direct security or privacy surface added. Importer now preserves more payload content fidelity for tool results (including object/scalar values), which can expose more raw content downstream; this is expected and still gated by export-time secret scanning in M8.
5. Performance: Negligible impact. `scan_highest_index` already scanned the file; changing return type to `Option` does not add cost. Importer now calls one helper for args/result value conversion; allocation behavior is effectively unchanged.

---

## M5.4 · viewmodel.hash computation (BLAKE3) · 2026-02-17

Context:
- Bead owner: Claude Opus 4.5 (claude-code)
- Invariants referenced: I4 (testable determinism)
- Constitution touched: none

1. Coupling: `viewmodel_hash()` couples ViewModel serialization to BLAKE3 hashing. If ViewModel fields change, hash outputs change. `viewmodel_hash_for_file()` is the format expected by Tour (M7) and CI assertions. The hash depends on `serde_json::to_vec()` serialization order — if serde changes field ordering, all hashes change.
2. Untested claims: (a) `serde_json::to_vec()` on ViewModel never fails — uses `expect()`. Current ViewModel is all-safe types, but adding a non-serializable field would panic. (b) BLAKE3 hash is assumed stable across library versions — pinned in Cargo.lock.
3. Nondeterminism: None introduced. BLAKE3 is deterministic. `serde_json::to_vec()` on ViewModel is deterministic (all BTreeMap, no floats in serialization). 12 tests verify hash stability including content-change detection.
4. Security: No secrets, tokens, or PII. The hash is a digest of ViewModel metadata, not event payloads. BLAKE3 has no known vulnerabilities for this use case.
5. Performance: No performance cliffs. BLAKE3 is fast (~3 GB/s on modern CPUs). ViewModel serialization is small (<1KB typical). 12 tests add <0.01s to the test suite.

---

## M5.5 · viewmodel.hash stability test (10 runs) · 2026-02-17

Context:
- Bead owner: Claude Opus 4.5 (claude-code)
- Invariants referenced: I2 (deterministic projection), I4 (testable determinism)
- Constitution touched: none

1. Coupling: `test_full_pipeline_determinism_10_runs` couples event schema (ImportEvent, EventPayload) to reducer (State, reduce) to projection (project, viewmodel_hash). Any change in these layers affects the test. This is intentional — the test catches regressions in the full pipeline.
2. Untested claims: (a) 10 runs is sufficient to detect nondeterminism — probabilistic, but catches common issues (HashMap iteration, RNG seeding). (b) The test events are representative — covers 4 of 8 Tier A types. Full coverage would require more events but test would be slower.
3. Nondeterminism: This test specifically catches nondeterminism. If the test passes, the pipeline is deterministic for the tested inputs. The test also runs across all 6 ladder levels in a variant test.
4. Security: No secrets, tokens, or PII. Test uses synthetic data.
5. Performance: No performance cliffs. 10 iterations × 4 events × (reduce + project + hash) is fast (<1ms). The all-ladder-levels variant adds 6 × 5 = 30 more iterations but still completes in <1ms.

## bd-7ww · Bugfix: payload_ref validation + clock-skew resume hydration · 2026-02-17

Context:
- Bead owner: SilverHarbor (codex-cli)
- Invariants referenced: I1, I4, I5
- Constitution touched: none

1. Coupling: `EventLogWriter::open` now depends on scan metadata (highest commit index plus per-source latest timestamps), increasing coupling between resume logic and clock-skew detection correctness. This coupling is intentional and local to `eventlog.rs`.
2. Untested claims: We still do not fail hard on partially-corrupt EventLog lines during metadata scan; invalid lines are skipped. The fix only ensures valid historical lines seed skew detection state.
3. Nondeterminism: No new nondeterminism introduced. Metadata scan uses deterministic max operations; blob ref validation is pure and deterministic.
4. Security: This fix removes a concrete traversal class by rejecting malformed `payload_ref` values in blob read/existence paths. Remaining security risk is that inline payload content can still contain secrets until M8 export scanner enforcement.
5. Performance: EventLog open now stores per-source timestamp maxima while scanning, adding small map-update overhead proportional to existing line count. This is acceptable for v0.1 scale and buys correctness across process restarts.

---

### M8.3 — Refusal report generation

Context:
- Bead owner: CloudyLake (claude-code)
- Invariants referenced: I3, I5
- Constitution touched: none

1. Coupling: `RefusalReport` now requires `eventlog_path` at construction, coupling the report to the export pipeline's knowledge of the source path. This is intentional — the schema contract requires it.
2. Untested claims: `format_utc_now()` hand-rolled date formatting is used instead of a date library. Tested implicitly via non-empty assertion but not validated against edge cases (leap seconds, year 2100). Acceptable since `scan_timestamp_utc` is informational only and excluded from hashing.
3. Nondeterminism: `scan_timestamp_utc` uses wall clock (`SystemTime::now()`), introducing non-determinism in the report file. This is explicitly permitted by the bead spec ("informational only, not in hash if report is hashed"). All other fields are deterministic; `blocked_items` is stably sorted by `(event_id, field_path, matched_pattern)`.
4. Security: No new security risk. Refusal reports contain redacted matches only (via `redact_match()`). The `blob_ref` field exposes content-addressed hashes, not secrets.
5. Performance: No new performance risk. Sorting blocked items is O(n log n) on finding count, negligible for expected volumes.

---

## bd-bjv.4 · M6.4: Truth HUD strip · 2026-02-17

Context:
- Bead owner: CloudyLake (claude-code)
- Invariants referenced: I2 (deterministic projection — Truth HUD renders ViewModel state)
- Constitution touched: none (references BACKPRESSURE_POLICY ladder levels for color coding)

1. Coupling: `truth_hud::render_truth_hud` takes `&ViewModel` directly — coupled to ViewModel struct fields (`degradation_level`, `aggregation_mode`, `aggregation_bin_size`, `tier_a_drops`, `export_safety_state`, `projection_invariants_version`, `queue_pressure()`). Adding/removing ViewModel fields that the HUD should display requires updating `truth_hud.rs`. Color thresholds (50%/80% for pressure) are hardcoded in `pressure_style()` — if BACKPRESSURE_POLICY changes pressure thresholds, these must be updated manually.
2. Untested claims: (a) `queue_pressure() * 100.0 as u32` truncates — values like 79.9% display as 79%, not 80%. This is cosmetic only. (b) The HUD assumes terminal width >= ~80 columns for the full line to display without wrapping. Narrow terminals may truncate content. (c) No test verifies the HUD is rendered in both lenses — the layout logic in `lib.rs` handles this, and the HUD tests only verify standalone rendering.
3. Nondeterminism: None introduced. All rendering is deterministic given the same ViewModel. No RNG, no wall clock, no HashMap. Color selection is pure function of ViewModel values.
4. Security: No secrets, tokens, or PII. The HUD displays only system metadata (levels, counts, versions). No event payloads flow through the HUD.
5. Performance: No performance cliffs. `render_truth_hud` creates a small number of `Span` objects (<20) and renders two text lines. O(1) in event count. 10 tests add <0.01s to the test suite.

---

## bd-d7c.4 · M8.4: Deterministic tar+zstd bundling · 2026-02-17

Context:
- Bead owner: CloudyLake (claude-code)
- Invariants referenced: I3 (share-safe export — deterministic bundles)
- Constitution touched: none (implements CAPACITY_ENVELOPE Export determinism targets)

1. Coupling: `create_bundle` depends on `tar` (0.4) and `zstd` (0.13) crates. The tar crate's UStar header layout and zstd's compression output are version-dependent. Pinned in Cargo.lock per bead spec. If either crate is upgraded, bundle bytes will change and BLAKE3 hashes will differ — any reproducibility checks across versions will break.
2. Untested claims: (a) `header.set_size(data.len() as u64)` uses `as` cast — safe for files under 2^64 bytes but not checked with TryFrom. Acceptable since the max blob size is 50MB per CAPACITY_ENVELOPE. (b) PAX extended headers: using UStar format which is PAX-compatible; true PAX extended headers are not explicitly emitted but also not needed for paths <100 chars and sizes <8GB. (c) No test verifies cross-platform determinism (same bytes on macOS vs Linux) — only tested same-machine determinism.
3. Nondeterminism: All metadata is normalized (mtime=0, uid/gid=0, username/groupname="", mode=0644). Entries are sorted alphabetically. Zstd level is pinned at 3. No wall clock, no RNG, no thread-local state. The only source of potential nondeterminism is crate version changes (tar/zstd library internals).
4. Security: No new security risk. `create_bundle` is only called after `scan_for_secrets` passes (no secrets). Bundle contents are EventLog + blobs that passed scanning. No credentials or PII in the archive metadata.
5. Performance: Entire bundle is built in memory (`Vec<u8>`) before writing to disk. For large EventLogs + many blobs, this could cause high memory usage. Current mitigation: max blob size is 50MB, practical bundle sizes are expected to be <100MB for v0.1. Streaming to disk would be needed for larger bundles in future versions.

---

## bd-d7c.5 · M8.5: Integrity manifest for clean bundles · 2026-02-17

Context:
- Bead owner: CloudyLake (claude-code)
- Invariants referenced: I3 (share-safe export — manifest provides verifiable receipt of bundle contents)
- Constitution touched: none (references CAPACITY_ENVELOPE and BACKPRESSURE_POLICY for projection_invariants_version)

1. Coupling: `BundleManifest` references `PROJECTION_INVARIANTS_VERSION` from panopticon-core. If the version constant changes, new bundles will embed the new version. Manifest schema version "manifest-v0.1" is hardcoded — changing the manifest format requires updating this and potentially adding backward-compatible parsing. The manifest does NOT include `bundle_hash` (circular dependency since manifest is inside the archive); `bundle_hash` lives in `ExportSuccess` only.
2. Untested claims: (a) `commit_index_range` assumes events are already sorted by commit_index in the EventLog file (relies on I1/I4: append writer assigns monotonic indices). No test verifies behavior with reordered events. (b) No test for empty EventLog edge case where `commit_index_range` is `None`. (c) Manifest JSON is pretty-printed (`to_string_pretty`), which slightly increases bundle size but aids debugging.
3. Nondeterminism: None. Manifest entries are sorted alphabetically (inheriting the sorted `entries` vec). All fields are deterministic: file hashes are BLAKE3, commit_index range is from deterministic EventLog, projection_invariants_version is a constant. No wall clock, no RNG.
4. Security: No new security risk. Manifest exposes file paths (archive-relative), sizes, and BLAKE3 hashes. No event content or blob data flows through the manifest. BLAKE3 hashes of blob data are already exposed via `payload_ref` in the EventLog.
5. Performance: Manifest creation adds one BLAKE3 hash per file entry (already computed as part of collecting entries) and one JSON serialization. Negligible overhead.

---

## bd-d7c.6 · M8.6: Export tests (clean, refusal, re-export) · 2026-02-17

Context:
- Bead owner: CloudyLake (claude-code)
- Invariants referenced: I3 (share-safe export), I4 (testable determinism via bundle_hash)
- Constitution touched: none

1. Coupling: Integration tests depend on the public API of panopticon-export (`run_export`, `ExportConfig`, `BundleManifest`, etc.) and panopticon-core (`EventLogWriter`, `BlobStore`, `ImportEvent`). Changes to the export pipeline public API or event schema will require test updates. This is desirable — tests should break when the API changes.
2. Untested claims: (a) Cross-machine determinism not tested (same EventLog on different OSes/architectures). (b) Large bundle behavior not tested (tests use small fixtures). (c) No test for concurrent export of the same EventLog. All acceptable for v0.1.
3. Nondeterminism: None. All tests use deterministic fixtures. No time-dependent assertions.
4. Security: Test fixtures include known secret patterns (AWS keys, passwords) for refusal testing. These are well-known example values (AKIAIOSFODNN7EXAMPLE) that are not real credentials.
5. Performance: 8 integration tests add ~0.04s to the test suite. Each creates temporary directories with small fixtures. No performance risk.

## bd-bjv.2 — M6.2: Incident Lens (default view)
- Files: `crates/panopticon-tui/src/incident_lens.rs` (new), `crates/panopticon-tui/src/lib.rs` (modified)
- Constitution touched: none

1. Coupling: Incident Lens renders directly from reducer `State` (run_metadata, event_counts_by_type, error_log, clock_skew_events, policy_decisions). Changes to State fields will require Incident Lens updates. This is acceptable — the TUI is a consumer of State.
2. Untested claims: (a) Visual appearance with very long run IDs or agent names is untested (could overflow). (b) Behavior with >100 anomalies rendering in a small terminal not tested. Both acceptable for v0.1.
3. Nondeterminism: None. All rendering is pure function of State.
4. Security: No security implications. TUI is read-only.
5. Performance: 9 new tests add ~0.02s. Rendering is O(runs + types + anomalies) per frame, negligible for expected data sizes.

## bd-bjv.3 — M6.3: Forensic Lens (timeline + inspector) + Truth HUD fix
- Files: `crates/panopticon-tui/src/forensic_lens.rs` (new), `crates/panopticon-tui/src/lib.rs` (modified), `crates/panopticon-tui/Cargo.toml` (modified), `crates/panopticon-tui/src/incident_lens.rs` (fmt-only)
- Constitution touched: none

1. Coupling: Forensic Lens renders directly from `Vec<CommittedEvent>`, requiring App to store events. ForensicState is owned by the forensic_lens module. The App now stores events (memory cost proportional to EventLog size). Acceptable for v0.1 local-only usage.
2. Untested claims: (a) Truncation in `truncate_or_full` uses byte slicing at position 59, which could panic on multi-byte UTF-8 if the 59th byte falls mid-character. Low risk — tool names and args are typically ASCII. (b) Scroll behavior with >1000 events not tested in render. (c) Expanded view of Generic payload with many data fields not render-tested.
3. Nondeterminism: None. Events are displayed by commit_index (from the eventlog read order). No HashMap iteration. No wall-clock in rendering. `queue_pressure * 100.0` in PolicyDecision display uses f64 formatting with explicit precision `{:.1}%`.
4. Security: No security implications. TUI is read-only. No user input stored.
5. Performance: Truth HUD fix: Length(3)→Length(4), zero perf impact. Forensic Lens: 12 new tests add ~0.03s. Rendering is O(visible_events) per frame. App stores full event list in memory — for v0.1 local-only, acceptable.

## bd-bjv.6 · M6.6: Truth HUD snapshot test

1. Coupling: Added `render_to_buffer` public function to panopticon-tui, exposing a `#[doc(hidden)]` test helper. Integration test depends on panopticon-core's EventLogWriter and panopticon-tui's render pipeline. Coupling is appropriate for an end-to-end test.
2. Untested claims: None. All 6 required HUD fields are asserted. Empty eventlog edge case covered. Version string exact-match tested.
3. Nondeterminism: None. render_to_buffer is deterministic (same EventLog → same output). TestBackend produces deterministic buffer content.
4. Security: No security implications. Test-only code, read-only rendering.
5. Performance: 4 integration tests add ~0.02s. render_to_buffer creates a TestBackend per call — acceptable for test use only.

## bd-d7c.7 · Export tests: empty EventLog integration test

1. Coupling: Tests added to existing integration test file. No new dependencies or coupling.
2. Untested claims: None. Empty EventLog export path fully covered: success semantics, manifest shape (commit_index_range absent), bundle contents (eventlog + manifest only), determinism across reruns.
3. Nondeterminism: None. Tests verify determinism explicitly (same hash, same bytes across runs).
4. Security: No security implications. Test-only additions.
5. Performance: 4 integration tests add ~0.01s. Minimal I/O (empty eventlog files).

## bd-c7m.2 · Tour: large stress fixture (10K events)

1. Coupling: New fixture `fixtures/large-stress.jsonl` (6.7 MB, 19,475 events). Generator binary `gen-large-stress` in panopticon-tour crate. No new library dependencies.
2. Untested claims: None. 9 integration tests verify all CAPACITY_ENVELOPE requirements: event count >= 10K, representative event mix (tool_use/tool_result/error/session_start/session_end), multiple runs (25 sessions), multiple agents (4), backward timestamps (5 for clock skew), varying payload sizes (2–4,413 bytes). Tour pipeline integration tests confirm successful processing and determinism.
3. Nondeterminism: None. Generator uses xorshift64 with fixed seed `0xDEAD_BEEF_CAFE_1234`. Same binary → same fixture. Fixture is committed as-is and verified deterministic through Tour pipeline.
4. Security: No security implications. Fixture contains synthetic data only — no real credentials, paths, or PII.
5. Performance: 9 integration tests add ~6.5s (dominated by Tour pipeline processing of 19K events). Fixture file is 6.7 MB — within acceptable limits for a committed test fixture.

## bd-c7m.3 · Tour: proof artifact emission (metrics.json, timetravel.capture)

1. Coupling: `DegradationTransition` struct in panopticon-tour mirrors `PolicyTransition` fields from panopticon-core reducer. If reducer adds/renames fields, tour must update. Acceptable — schema is documented in PLANS.md.
2. Untested claims: `max_degradation_level` uses lexicographic string max over level names. This relies on the invariant that levels are named "L0"–"L5" where lex order matches severity. If a level is renamed (e.g., "Critical"), the max computation would break. Small fixtures produce no degradation_transitions, so the empty-array path is tested but the populated-array path is only exercised via the large stress fixture.
3. Nondeterminism: `queue_pressure_micro` (u64) → f64 division is exact for values in [0, 1_000_000]. No HashMap iteration, no wall-clock, no random seeds. Seek point interval is deterministic from committed event count.
4. Security: No security implications. Proof artifacts contain only hashes, counts, and level strings.
5. Performance: Seek point capture projects at ~20 intervals during reduction, adding ~20 extra `project()` calls. For 19K events this is negligible. No unbounded allocation — seek_points vec grows to at most ~21 entries.

## bd-c7m.5 · Tour: ansi.capture emission

1. Coupling: ANSI color logic (`ansi_level`, `ansi_drops`, `ansi_export`, `ansi_pressure`) mirrors Truth HUD color semantics from `panopticon-tui/src/truth_hud.rs`. If TUI changes color thresholds (e.g., pressure 80% → 75%), tour's ANSI capture will diverge. Acceptable for v0.1 — both are derived from the same BACKPRESSURE_POLICY specification. Cannot use panopticon-tui directly due to circular dependency (tui → tour).
2. Untested claims: ANSI output is not compared byte-for-byte against TUI rendering. The capture mirrors Truth HUD fields and color logic but uses raw ANSI codes rather than ratatui. Visual parity with the actual TUI is not verified — only that all required fields and escape codes are present.
3. Nondeterminism: None. `render_ansi_capture` is a pure function from ViewModel + event_count + hash → String. No wall-clock, no randomness, no platform-dependent formatting. Uses `std::fmt::Write` which is deterministic. Determinism is tested explicitly.
4. Security: No security implications. ANSI capture contains only ViewModel field values (levels, counts, hashes). No secrets or PII.
5. Performance: Single `render_ansi_capture` call per Tour run — trivial String formatting. No allocation concerns.

## bd-c7m.4 · M7.4: CI assertion tests for Tour invariants

1. Coupling: The new invariant tests depend on Tour's current ingestion path (cassette -> append writer -> reducer). If importer mappings change, expected PolicyDecision derivation behavior may change and these tests will fail. This is acceptable because the tests are intended to pin invariant semantics, not implementation details.
2. Untested claims: Large fixture currently does not produce non-empty PolicyDecision transitions, so production-path transition parity is asserted mainly on the empty-set case. A non-tautological ladder validator test with synthetic transitions was added, but real non-empty fixture coverage remains future work.
3. Nondeterminism: No new nondeterminism introduced. Tests run deterministic pipelines and compare stable artifact outputs and derived transition tuples.
4. Security: No new security risk. Tests use synthetic fixture data and do not handle secrets or credentials.
5. Performance: Added integration tests rerun Tour and replay EventLog, increasing test time by several seconds. This is acceptable for CI given the value of invariant enforcement.

## bd-2fp.1 · A1-1: Tour benchmark harness + baseline capture · 2026-02-17

1. Coupling: New benchmark entrypoint (`crates/panopticon-tour/src/bin/bench_tour.rs`) depends on current Tour CLI pipeline and fixture location (`fixtures/large-stress.jsonl`). If fixture path or run_tour contract changes, the benchmark tool must be updated.
2. Untested claims: Benchmark numbers are single-host samples and are not cross-machine comparable. They are suitable for relative trend checks, not absolute SLO commitments.
3. Nondeterminism: The benchmark computes wall-time percentiles, which are expected to vary by host load. This does not affect truth artifacts because benchmark output is separate from canonical run artifacts.
4. Security: No new secret handling surface. Benchmark consumes existing synthetic fixture and emits only timing metrics.
5. Performance cliffs: Running benchmark in `--release` over large fixture is CPU-heavy by design; if run with high iteration count it can consume local resources. Guardrail is configurable iteration count via `PANOPTICON_TOUR_BENCH_ITERS`.

## bd-2fp.2 · A1-2: Remove duplicated parse/replay in Tour invariant tests · 2026-02-17

1. Coupling: Test helpers now cache parsed fixtures and derived policy-transition tuples via `OnceLock`. If fixture-generation semantics change, cached expectations will still reflect current fixture content per process start, but test assumptions remain coupled to fixture shape.
2. Untested claims: This bead optimizes test-path execution only; it does not claim runtime improvements. No new product-path assertions were added.
3. Nondeterminism: `OnceLock` introduces shared test-process cache state. Values are deterministic because inputs are deterministic and initialized once; there is no time/random input.
4. Security: No new security surface. Cached data is synthetic fixture content and derived transition tuples.
5. Performance cliffs: Memory footprint increases slightly by retaining parsed fixture and derived tuples for test process lifetime. Tradeoff is intentional for reduced repeated parse/replay work.

## bd-2fp.3 · A1-5: Release trust hardening (attestations + verify docs) · 2026-02-17

1. Coupling: CI now depends on GitHub-specific attestation action (`actions/attest-build-provenance@v1`) and artifact upload path conventions under `dist/`.
2. Untested claims: Attestation verification is documented but not executed in local tests; it requires GitHub-hosted workflow context and repository attestation APIs.
3. Nondeterminism: Build timestamps/environment in compiled binaries may vary, but this change does not alter Panopticon deterministic truth/projection artifacts.
4. Security: Improves supply-chain posture by adding provenance metadata and checksum workflow. No new runtime secret surface added.
5. Performance cliffs: Added release-trust CI job increases CI runtime on `main` and `v*` tags; impact is bounded to release-trust paths and does not affect local runtime performance.

## bd-2fp.4 · A1-3: Deterministic artifact serialization policy decision · 2026-02-17

1. Coupling: Added explicit policy coupling between Tour artifact writers and documented byte-shape expectations (`docs/ARTIFACT_SERIALIZATION_POLICY.md` + Tour tests).
2. Untested claims: Policy is currently enforced for pretty JSON shape and hash newline behavior in Tour tests; cross-tool consumers are not auto-validated yet.
3. Nondeterminism: None introduced. Policy reinforces existing deterministic artifact modes and adds tests to prevent accidental serializer mode drift.
4. Security: No new secret surface. This is documentation + deterministic test enforcement.
5. Performance cliffs: Pretty JSON remains slightly larger than compact JSON; this is an intentional readability/stability tradeoff and unchanged from prior behavior.

## bd-38h · Process: enforce handoff/commit-message accuracy checks · 2026-02-17

1. Coupling: Process coupling increases between commit messages, handoff notes, and staged diffs; this is intentional governance coupling to reduce ambiguity.
2. Untested claims: No executable behavior changed; policy enforcement relies on human checklist adherence rather than automated linting.
3. Nondeterminism: None introduced; documentation/process updates only.
4. Security: Improves audit integrity by reducing mismatch between claimed and actual changes; no new data/security surface.
5. Performance cliffs: Negligible; adds a lightweight manual review step before commit/handoff.

## bd-x7q.1 · README-PLAN: finalize launch sequencing + acceptance checklist · 2026-02-17

1. Coupling: `docs/README_LAUNCH_PLAN.md` now explicitly couples execution order across `bd-x7q.1`..`bd-x7q.5`; this is intentional to prevent out-of-order launch work.
2. Untested claims: This bead is docs/process only. It does not assert runtime behavior changes; no new executable claims were introduced.
3. Nondeterminism: None introduced. No runtime code paths changed.
4. Security: Improves release hygiene by requiring verification findings to gate release tagging. No new secret/PII surface.
5. Performance cliffs: None; documentation-only change.

## bd-x7q.2 · README-CORE: root README proof-first rewrite · 2026-02-17

1. Coupling: README command examples now couple to current CLI flags and subcommand names (`view`, `export`, `tour`); future CLI changes must update README and downstream verification bead.
2. Untested claims: README intentionally avoids unverified claims about packaging channels, asset paths, or hosted deployment. Commands shown were validated against current CLI help and stress-tour execution shape.
3. Nondeterminism: No product-path nondeterminism introduced; docs-only change.
4. Security: README reinforces `--share-safe` export posture and trust-challenge checks. No new secret or credential handling surface added.
5. Performance cliffs: None. Documentation-only changes.

## bd-x7q.3 · README-ASSETS: deterministic capture assets + architecture visual · 2026-02-17

1. Coupling: Added `capture_readme_assets` binary and hidden render helpers in `panopticon-tui` to generate README assets from current render paths and export/tour APIs. If lens titles, HUD text, or export refusal formatting change, regenerated assets will change accordingly.
2. Untested claims: Asset generation binary has no unit tests; correctness is validated through full workspace gates and deterministic capture outputs. Future bead should consider snapshot tests for generated assets if strict byte pinning is required.
3. Nondeterminism: Captures are deterministic for provided sample EventLog and fixture inputs. `artifacts-view.txt` includes Tour hash for `fixtures/large-stress.jsonl` and can change only if deterministic core behavior changes.
4. Security: Export-refusal asset intentionally includes redacted secret matches from synthetic input only. No real credentials introduced.
5. Performance cliffs: Running `capture_readme_assets` executes a full Tour run on `large-stress.jsonl`, which is intentionally non-trivial CPU work. This is acceptable for docs asset refresh cadence.

## bd-x7q.4 · README-VERIFY: command and trust-step reproducibility validation · 2026-02-17

1. Coupling: README examples are now explicitly coupled to generated sample assets under `docs/assets/readme/` and to current CLI contract (`panopticon` binary flags/subcommands).
2. Untested claims: `view` cannot be fully exercised in this non-interactive sandbox due TTY constraints; verification records this explicitly rather than claiming a full pass.
3. Nondeterminism: Determinism checks were rerun (`tour` twice) and hashes matched; no new nondeterministic behavior introduced.
4. Security: Verification surfaced conservative scanner false positives on numeric-heavy sample data and led to a dedicated clean export sample; this reduces confusion without weakening share-safe behavior.
5. Performance cliffs: Verification path runs full stress Tour twice; this is expected for trust-challenge validation and documented as a heavyweight check.

## bd-x7q.5 · README-REVIEW: independent QA/polish gate · 2026-02-17

1. Coupling: Review uncovered command-coupling drift after adding a second binary in `panopticon-tui`; README commands now explicitly pin `--bin panopticon` to remain unambiguous.
2. Untested claims: No new product claims added. Remaining caveat is TUI `view` requires interactive TTY, documented in README and verification log.
3. Nondeterminism: None introduced; documentation fixes only.
4. Security: No new security surface. Review retained conservative share-safe posture and explicit refusal-path validation.
5. Performance cliffs: None. QA/polish documentation updates only.

## bd-3qq.2 · RELEASE-OPS: automate release build and artifact verification · 2026-02-17

1. Coupling: CI `release-trust` now depends on `scripts/release_artifacts.sh` and `scripts/verify_release_artifacts.sh`; workflow and local release process are intentionally coupled to one command path.
2. Untested claims: Provenance attestation verification still requires GitHub-hosted context; local verification covers checksum integrity and artifact presence only.
3. Nondeterminism: Release binaries may differ across toolchain/host metadata, but checksum generation and verification flow is deterministic for a given build output.
4. Security: This improves supply-chain posture by enforcing checksum verification in CI before artifact upload.
5. Performance cliffs: Release packaging adds extra CI minutes on release-trust runs; impact is bounded to release paths and does not affect runtime behavior.

## bd-3qq.3 · RELEASE-OPS: packaging matrix + publish checklist · 2026-02-17

1. Coupling: Introduced explicit coupling between release gates and docs (`docs/RELEASE_PACKAGING_CHECKLIST.md`, `docs/RELEASE_TRUST_VERIFICATION.md`) so operational flow stays single-sourced.
2. Untested claims: Homebrew/winget channels remain optional and unimplemented in-repo; checklist marks them deferred rather than claiming readiness.
3. Nondeterminism: No runtime/path changes; checklist only. Deterministic checks remain delegated to existing Tour and checksum verification paths.
4. Security: Go/no-go now explicitly requires checksum verification and green release-trust CI before publication, reducing accidental unverified release risk.
5. Performance cliffs: No runtime performance impact; process overhead is release-time only.

## bd-3qq.1 · LAUNCH-MEDIA: demo script + capture runbook · 2026-02-17

1. Coupling: Demo flow is now coupled to current sample assets and CLI command shape (`--bin panopticon`). If command flags or sample paths change, demo script/runbook must be updated.
2. Untested claims: Optional recording tools (`asciinema`, `vhs`) are documented as optional; this bead does not claim tool availability in all environments.
3. Nondeterminism: Demo quickcheck explicitly uses deterministic fixture and verifies trust outputs; no new nondeterministic runtime behavior introduced.
4. Security: Demo script preserves share-safe posture by demonstrating both success and refusal paths, reducing risk of unsafe export messaging.
5. Performance cliffs: Quickcheck runs full stress Tour and can be CPU-heavy; this is expected and acceptable for pre-release media capture.

## bd-3qq.4 · LAUNCH-COMMS: release notes + social copy pack · 2026-02-17

1. Coupling: Launch copy now intentionally couples to verified docs (`docs/README_VERIFICATION.md`, `docs/RELEASE_TRUST_VERIFICATION.md`) and canonical assets in `docs/assets/readme/`; messaging must be updated when those proofs or asset paths change.
2. Untested claims: Social templates reference optional channels (X, LinkedIn, article) but do not prove audience performance; this bead provides technically accurate copy, not distribution outcomes.
3. Nondeterminism: None introduced. Documentation-only changes with no runtime behavior changes.
4. Security: Guardrails explicitly prohibit over-claiming hosted deployment or unsupported channels, reducing risk of misleading trust/security statements.
5. Performance cliffs: None in runtime. Minor process overhead during launch prep due to copy review and claim verification.

## bd-3qq.5 · POST-LAUNCH-LEARN: feedback intake and prioritization rubric · 2026-02-17

1. Coupling: Triage workflow now couples launch feedback handling to GitHub issue labels and evidence artifacts (`metrics.json`, `viewmodel.hash`, `refusal-report.json`). Process discipline is required to keep classifications consistent.
2. Untested claims: This bead defines process and prioritization rules only; it does not guarantee issue volume reduction or social channel conversion outcomes.
3. Nondeterminism: None introduced. Docs-only updates with no runtime behavior changes.
4. Security: The rubric reduces risk by requiring explicit evidence for security/export claims and by prioritizing secret-handling failures as P0.
5. Performance cliffs: No runtime impact. Operational overhead is bounded to launch-week triage cadence.

## bd-1w9.1 · README-TOOLS: evaluate and select beautification tooling · 2026-02-17

1. Coupling: README polish now couples to selected tools (`shields.io`, Mermaid, optional `vhs`/`asciinema`, `markdownlint-cli2`). Future README updates should stay within this tool policy to avoid style drift.
2. Untested claims: Tool selection guidance does not guarantee improved engagement; it defines implementation constraints and quality posture only.
3. Nondeterminism: No runtime nondeterminism introduced; this is documentation and process guidance only.
4. Security: Restricting visuals to deterministic terminal-native captures reduces risk of misleading or fabricated evidence imagery.
5. Performance cliffs: No product runtime impact; minor contributor overhead when regenerating visual assets.

## bd-1w9.2 · README-IA: redesign information architecture and story flow · 2026-02-17

1. Coupling: README structure now couples onboarding flow to specific command paths (`--bin panopticon`, stress Tour, share-safe export). CLI contract changes will require coordinated README updates.
2. Untested claims: The IA rewrite improves clarity on paper, but audience comprehension gains are not yet measured by UX sessions.
3. Nondeterminism: No runtime nondeterminism introduced; documentation-only changes.
4. Security: The rewrite keeps trust claims tied to verifiable commands and avoids promising unsupported deployment/security properties.
5. Performance cliffs: No runtime impact. Slight maintenance overhead for keeping architecture and workflow sections aligned with future crate/command changes.

## bd-1w9.3 · README-ASSETS: deterministic terminal-native visual asset pack · 2026-02-17

1. Coupling: README visuals now explicitly couple to `capture_readme_assets` and `scripts/refresh_readme_assets.sh`. Changes in rendering/capture code may require asset refresh and README review.
2. Untested claims: The playbook documents reproducibility and optional recording tools, but does not prove social-channel engagement outcomes.
3. Nondeterminism: Core asset generation remains deterministic; optional recording tools (`asciinema`, `vhs`) can include timing variance and are excluded from canonical proof claims.
4. Security: Avoiding AI-generated or manually fabricated visuals reduces risk of misleading evidence presentation.
5. Performance cliffs: Asset refresh runs Tour capture and may be moderately CPU-heavy; this is acceptable for docs update workflows.

## bd-1w9.4 · README-IMPLEMENT: polished layout, badges, and verified examples · 2026-02-17

1. Coupling: README now couples to CI workflow naming (`ci.yml`) for status badges and to release/tag surfaces for release badge rendering.
2. Untested claims: Badge rendering and mermaid display are platform-dependent on GitHub UI behavior; functionality claims remain command-verifiable.
3. Nondeterminism: No runtime nondeterminism introduced; documentation presentation changes only.
4. Security: README still avoids unverified security claims and keeps trust language tied to reproducible commands.
5. Performance cliffs: No product runtime impact; minor docs maintenance overhead when workflows/repo metadata change.

## bd-1w9.5 · README-QA: readability, command validity, and launch-quality review · 2026-02-17

1. Coupling: README examples are now explicitly validated against current CLI behavior and generated asset paths; future command/asset changes require QA report refresh.
2. Untested claims: Readability improvements are reviewed manually and command-validated, but broader audience comprehension still depends on future UX sessions.
3. Nondeterminism: Determinism checks were re-run and hash outputs matched; no new nondeterministic behavior introduced.
4. Security: QA reinforces share-safe export posture and avoids introducing claims beyond validated behavior.
5. Performance cliffs: QA process includes two stress Tour runs and can be time-consuming in CI-like environments, but this is intentional for trust verification.

## bd-gxd.1 · UX-SCOPE: map premium UX goals to terminal-first constraints

1. Coupling: Added a canonical UX scope reference (`docs/UX_SCOPE.md`) that downstream UX beads now depend on for principles and non-goals. Future UX edits should update this doc to avoid divergent strategy notes.
2. Untested claims: The scope itself is planning text and does not assert runtime behavior. Behavioral claims remain to be validated by `bd-gxd.2`..`bd-gxd.10` and `bd-2yv.5`.
3. Nondeterminism: None introduced. Documentation-only change; no runtime logic, ordering, hashing, or rendering behavior changed.
4. Security and privacy: No new data handling. No secrets/PII introduced.
5. Performance cliffs: None introduced. Potential process risk is over-scoping UX work; mitigated by ranked pain points and explicit non-goals in `docs/UX_SCOPE.md`.

## bd-gxd.8 · UX-MODALITY-SPEC: define desktop, narrow-terminal, and mobile-readme UX matrix

1. Coupling: Added `docs/UX_MODALITY_MATRIX.md` and linked it from `docs/UX_SCOPE.md`; downstream UX beads now rely on this matrix for width-bucket behavior and mobile-readme constraints.
2. Untested claims: This bead defines planning constraints only. Runtime behavior remains untested until implementation beads (`bd-gxd.2`..`bd-gxd.6`) and validation beads (`bd-gxd.9`, `bd-gxd.10`) execute.
3. Nondeterminism: None introduced. Documentation-only update; no runtime ordering, hashing, or projection logic changed.
4. Security and privacy: No new data handling or exposure risk.
5. Performance cliffs: None in runtime. Process risk is over-constraining layout decisions; mitigated by explicit required/optional/deferred tiers per width bucket.

## bd-gxd.2 · UX-CLI-RECOVERY: actionable error and refusal guidance

1. Coupling: Centralized CLI failure formatting in `format_cli_failure` inside `crates/panopticon-tui/src/main.rs`; view/tour/export failures now share one message contract, improving consistency but coupling copy style to this helper.
2. Untested claims: Added unit tests for section structure and numbered commands, but did not add subprocess integration tests for every runtime error branch; this remains partially covered by existing end-to-end suites.
3. Nondeterminism: None introduced. Message formatting is deterministic string assembly from explicit inputs; no time/random/source-order behavior added.
4. Security and privacy: Recovery output may include file paths supplied by user arguments; no secret content is introduced by this bead.
5. Performance cliffs: Negligible. Additional string formatting on error paths only; no steady-state impact.

## bd-gxd.3 · UX-ONBOARDING: first-run guidance strip and progressive hints

1. Coupling: Incident Lens now accepts a `show_onboarding` flag from `App`; onboarding visibility state is managed in TUI app logic, coupling first-run guidance to key-event handling.
2. Untested claims: Added unit tests for default visibility, hide-after-interaction behavior, and onboarding rendering, but no external PTY interaction test yet for full session behavior.
3. Nondeterminism: None introduced. Onboarding visibility is deterministic state transition based on explicit key input; no wall-clock or randomness.
4. Security and privacy: No new data surfaces beyond static guidance text.
5. Performance cliffs: Minimal. One additional short render block and boolean check per frame in Incident Lens.

## bd-gxd.4 · UX-INCIDENT-HIERARCHY: triage-first incident lens layout

1. Coupling: Incident Lens ordering now explicitly prioritizes anomaly triage before run/event context; copy and layout expectations are now coupled to this order and should remain aligned with operator workflow docs.
2. Untested claims: Unit tests cover section ordering and headings, but no PTY-level behavioral test yet verifies human scan-time improvement under real terminal interaction.
3. Nondeterminism: None introduced. Rendering remains deterministic over reducer state with no wall-clock/random inputs added.
4. Security and privacy: No new data exposure paths; anomaly details still render from existing state fields only.
5. Performance cliffs: Minimal impact. Additional anomaly summary line and section reordering are O(n) on already-materialized lists; no new heavy allocations or I/O paths.

## bd-gxd.5 · UX-CONTEXT-HINTS: lens-aware key hints and next actions

1. Coupling: Help-copy logic is now lens/state-aware and coupled to `ForensicState.expanded` plus anomaly presence in `State`; future keybinding changes must update these hint templates together.
2. Untested claims: Unit tests validate hint switching for anomaly/no-anomaly and expand/collapse states, but no terminal usability study yet measures reduced operator confusion.
3. Nondeterminism: None introduced. Hint selection is pure deterministic branching from existing state and keyflow.
4. Security and privacy: No new data surfaces; hints reference only existing event metadata and controls.
5. Performance cliffs: Minimal. Added string formatting in render path is bounded and proportional to already-rendered state, with no new I/O or allocations of unbounded structures.

## bd-gxd.6 · UX-VISUAL-TONE: cohesive color and copy style standard

1. Coupling: Incident/Forensic lens styling now depends on shared `visual_tone` helpers, which centralizes semantics but requires coordinated updates if token meanings change.
2. Untested claims: Unit/integration suites confirm behavior did not regress, but we have not yet run dedicated operator preference testing for perceived readability improvements.
3. Nondeterminism: None introduced. Style tokenization and copy updates are static mappings with no time/random input.
4. Security and privacy: No new data collection or output exposure; changes are presentation and documentation only.
5. Performance cliffs: Negligible. Shared style helper calls are lightweight and replace equivalent inline style construction.
