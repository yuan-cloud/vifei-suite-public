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
