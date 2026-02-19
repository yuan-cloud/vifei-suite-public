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

## bd-2cj9.1 · E1: masking-pattern audit (last-5-commits + HEAD) · 2026-02-18

Context:
- Bead owner: Codex GPT-5
- Invariants referenced: I4, I5
- Constitution touched: none

1. Coupling: The audit introduces process coupling between CLI/runtime changes and explicit masking-risk review criteria. This is intentional and scoped to TRACK-E.
2. Untested claims: The audit itself does not execute runtime fault-injection paths; it classifies code patterns and commit diffs. Follow-up beads (`bd-2cj9.2`, `bd-2cj9.3`) own executable proof.
3. Nondeterminism: No runtime behavior changed in this bead. The audit identified one remaining nondeterminism-adjacent masking risk (`delta.rs` payload serialization fallback) for explicit remediation.
4. Security: No new secret/PII surface introduced. The audit reduces security/quality risk by identifying where silent fallbacks can hide failures.
5. Performance: No performance impact from this bead; documentation-only changes.
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

## bd-2yv.1 · TEST-AUDIT: coverage baseline and gap matrix

1. Coupling: The new baseline matrix now couples downstream testing beads (`bd-2yv.2`..`bd-2yv.8`, `bd-gxd.9`, `bd-gxd.10`) to a shared gap taxonomy; future test work should update this matrix to prevent drift.
2. Untested claims: Numeric line/function percentages are not available yet because `cargo llvm-cov` is not installed; this audit uses a command-backed inventory fallback and explicit risk ranking.
3. Nondeterminism: None introduced in product behavior; documentation-only audit artifact.
4. Security and privacy: No secret handling changes. Commands and findings reference existing local fixtures and test targets.
5. Performance cliffs: No runtime impact. Audit command set is heavier than smoke tests and should remain in planning/release lanes, not every edit loop.

## bd-2yv.2 · TEST-E2E-CLI: end-to-end script suite with detailed structured logs

1. Coupling: The new CLI e2e harness couples to current command UX text (`Export successful`, `export refused`, `Likely cause`) and artifact names; command copy contract changes must update script assertions.
2. Untested claims: Script currently validates refusal via stderr contract rather than guaranteed refusal-report file presence; refusal-report persistence behavior should be tightened in a follow-on bead.
3. Nondeterminism: Structured log format is deterministic (`run_id`, monotonic `seq`, stable fields). Runtime command output may vary in timing but is captured as transcripts.
4. Security and privacy: Uses repo fixtures only; refusal transcript may include masked secret indicators by design.
5. Performance cliffs: Script runs full stress tour and export flows, so it is not suitable for every edit loop; intended for e2e lane usage.

## bd-2yv.3 · TEST-CORE: close no-mock unit/integration gaps in core crates

1. Coupling: Added direct tests around `capture_readme_assets` generation helpers; future changes to README artifact names and refusal formatting now require synchronized test updates.
2. Untested claims: Core determinism and structure are covered for the capture binary helpers, but operator-level visual quality of generated artifacts still depends on higher-level acceptance checks.
3. Nondeterminism: Fixed a real nondeterminism source by removing stale `sample-refusal-eventlog.jsonl` before regenerating refusal artifacts; repeat runs now produce stable refusal output.
4. Security and privacy: Refusal test fixtures intentionally include secret-like markers for scanner assertions only; no new secret exposure paths were introduced.
5. Performance cliffs: Added unit tests are lightweight and file-local; no runtime production performance impact.

## bd-2yv.4 · TEST-E2E-TUI: interactive TUI end-to-end harness

1. Coupling: PTY E2E assertions now couple to visible TUI labels (Incident/Forensic lens and Truth HUD markers), so intentional UX copy changes must update this harness.
2. Untested claims: True interactive key handling is now exercised; however, environments without PTY support follow explicit skip policy and rely on existing unit/snapshot coverage.
3. Nondeterminism: Added bounded retry (max 1) and deterministic key sequences; preflight gate avoids flaky hard-fails in PTY-restricted environments.
4. Security and privacy: Harness uses synthetic local fixtures only and writes transcripts to local `.tmp` output paths for failure triage.
5. Performance cliffs: New PTY tests are lightweight but add external process overhead; they are scoped to one flow test plus one narrow-terminal profile.

## bd-2yv.5 · TEST-UX: operator usability protocol and scoring

1. Coupling: UX validation now depends on explicit task/script and scoring contracts in `docs/UX_TEST_PLAN.md`; downstream UX/test beads should use this protocol to avoid divergent evaluation criteria.
2. Untested claims: Baseline pass confirms CLI trust/recovery flows and captures PTY skip behavior, but does not yet validate real interactive PTY execution on this host due environment limits.
3. Nondeterminism: Protocol uses deterministic command scripts and fixed fixture inputs; PTY-dependent tasks include explicit skip reasons to avoid flaky implicit failures.
4. Security and privacy: Evidence artifacts are local test outputs only; no production secrets were introduced.
5. Performance cliffs: Baseline protocol includes stress tour and can be heavier than inner-loop tests, so it should run in validation lanes rather than every edit step.

## bd-gxd.9 · UX-MODALITY-VALIDATION: execute width-bucket and mobile-readability validation

1. Coupling: Added automated modality checks (`modality_validation` integration test) that couple UX contract labels and README section/readability constraints to explicit assertions.
2. Untested claims: Interactive PTY behavior still depends on host PTY capability; in constrained environments this remains skip-based and tracked via dedicated follow-up beads.
3. Nondeterminism: Validation runs are deterministic over fixed fixtures and explicit width buckets; no time/random inputs are used in new assertions.
4. Security and privacy: Validation artifacts are local and fixture-based, with no secret-bearing production data.
5. Performance cliffs: Added tests increase validation surface slightly but remain lightweight relative to existing full test suite.

## bd-gxd.10 · UX-EVIDENCE-ASSETS: deterministic UX captures for README and release proof

1. Coupling: README evidence assets are now explicitly coupled to the capture pipeline contract, including the new narrow capture artifact (`incident-lens-narrow-72.txt`).
2. Untested claims: Determinism is validated via two-pass hash comparison for normalized asset set; `refusal-report.json` remains intentionally variable due export metadata and is documented as excluded.
3. Nondeterminism: Removed append-based drift by resetting sample eventlog files before regeneration; this closes a real deterministic-capture defect.
4. Security and privacy: Evidence artifacts are fixture-derived and local; refusal examples intentionally include redacted secret patterns for scanner behavior proof.
5. Performance cliffs: Refresh pipeline now produces one additional render artifact; overhead is negligible compared with tour generation already in the pipeline.

## bd-2yv.8 · TEST-FASTLANE: sub-5-minute smoke suite for developer feedback

1. Coupling: Fastlane now couples CI and local smoke checks to specific stage names, output file paths, and CLI/TUI contract strings (`Export successful`, `export refused`, `Likely cause`), so intentional UX/copy changes must update fastlane assertions.
2. Untested claims: Fastlane covers deterministic smoke only; it does not replace full-suite behavioral depth (broad combinatorial cases and longer stress envelopes) and should remain a gate pre-check, not final release evidence.
3. Nondeterminism: No new product-path nondeterminism was introduced; fastlane logs are sequence-numbered and stable-field JSONL, and assertions use deterministic fixtures and explicit command contracts.
4. Security and privacy: The lane exercises share-safe refusal fixtures and captures transcripts under `.tmp/fastlane`; these logs can include refusal reasons and should remain local CI artifacts, not public release assets.
5. Performance cliffs: Fastlane budget is bounded (`<=300s`) with explicit fail-on-budget breach, but cargo cache misses in cold CI can approach the ceiling; fallback full-suite commands are documented for triage when this happens.

## bd-2yv.7 · TEST-DEFER-REGISTER: explicit uncovered-path waiver ledger

1. Coupling: CI now couples test governance to `docs/testing/defer-register-v0.1.json` schema and validator behavior; any ledger shape changes must update `scripts/testing/validate_defer_register.py` in lockstep.
2. Untested claims: The validator enforces field/date integrity and expiry checks, but it does not yet auto-verify that each `linked_beads` ID currently exists in `.beads/issues.jsonl`.
3. Nondeterminism: No runtime nondeterminism introduced in product code. Validation uses deterministic JSON parsing and explicit date comparisons.
4. Security and privacy: Ledger content is metadata-only and should not include secrets. CI enforcement reduces risk of silent stale waivers, but human review is still required to keep rationales accurate.
5. Performance cliffs: Negligible. Validator cost is O(n) in waiver entries and runs quickly in CI; no production-path overhead.

## bd-2yv.6 · TEST-CI-GATE: enforce coverage and E2E suites in CI

1. Coupling: CI release gating is now explicitly coupled to job topology (`fastlane`, `full-confidence`, `release-trust`) and artifact paths under `.tmp/full-confidence`; workflow changes must keep docs and paths synchronized.
2. Untested claims: We validated workflow logic locally via command parity, but did not execute GitHub-hosted jobs in this session; first remote run should be reviewed to confirm artifact upload paths and log retention behavior.
3. Nondeterminism: No product-path nondeterminism introduced. CI evidence generation uses deterministic command lists and fixed output locations.
4. Security and privacy: Uploaded CI artifacts now include richer test transcripts; they are fixture-based in this repo but still require normal repository access controls and retention discipline.
5. Performance cliffs: `full-confidence` is intentionally heavier than PR fastlane and can increase push CI duration/cost; this is mitigated by keeping `fastlane` as PR default and using evidence artifacts for fast failure triage.

## bd-gxd.7 · UX-VALIDATION: run operator-task polish validation and convert findings

1. Coupling: Validation output is now coupled to the current UX evidence document set and naming (`ux-baseline`, `ux-modality-validation`, `ux-evidence-refresh`); future report renames should be coordinated.
2. Untested claims: Interactive PTY triage behavior remains environment-dependent and may SKIP on hosts lacking PTY permissions; this is tracked by follow-up beads (`bd-kko`, `bd-1un`).
3. Nondeterminism: No runtime nondeterminism introduced; this bead is report/coordination only.
4. Security and privacy: Validation evidence uses local fixtures and operational metadata only; no new secret-bearing surfaces added.
5. Performance cliffs: None in product code. Process overhead is limited to periodic validation report updates.

## bd-gxd · UX-POLISH: premium operator experience for Panopticon CLI/TUI

1. Coupling: The UX polish track now depends on maintaining consistency between CLI recovery copy, lens-specific hints, onboarding behavior, and modality evidence docs.
2. Untested claims: Some UX outcomes are measured via deterministic proxy checks rather than broad human studies; repeat operator sessions should continue before major UX claims are expanded.
3. Nondeterminism: The track preserved deterministic rendering and hash invariants; no truth-path randomness or time-based ordering was introduced.
4. Security and privacy: UX changes stayed within existing share-safe and refusal messaging boundaries; no additional export or secret handling risk was introduced.
5. Performance cliffs: Presentation-level additions are lightweight; the main risk remains CI/runtime validation overhead, already bounded by fastlane/full-confidence split.

## bd-2yv · TEST-HARDEN: full coverage map + no-mock E2E and UI validation

1. Coupling: Test-hardening now tightly couples release readiness to CI lane topology and evidence artifact contracts; future workflow changes must preserve these interfaces.
2. Untested claims: At bead close time, numeric line/function coverage was deferred pending llvm-cov availability; this was later retired by `bd-im6j`, and no active numeric-coverage waiver remains.
3. Nondeterminism: Determinism checks were strengthened (including capture pipeline fixes), with remaining intentional variability documented (`refusal-report.json` metadata path).
4. Security and privacy: Structured logs improve triage but may contain operational refusal context; artifact access controls remain important.
5. Performance cliffs: Full-confidence CI is intentionally heavier; fastlane remains the mitigation for developer feedback latency.

## bd-kko · TEST-E2E-PTY-PATH: normalize TUI e2e output path to workspace root

1. Coupling: TUI E2E output path resolution now depends on workspace root derivation from `CARGO_MANIFEST_DIR` (two-level parent assumption), which should remain stable with current workspace layout.
2. Untested claims: We added direct test coverage for relative path normalization and exercised interactive tests, but cross-platform path behavior (non-Unix separators) still relies on Rust `PathBuf` semantics and has not been separately profiled.
3. Nondeterminism: No nondeterminism introduced; path normalization is deterministic and independent of runtime clock/randomness.
4. Security and privacy: No new secret/data exposure paths. Artifact location changed to improve traceability only.
5. Performance cliffs: Negligible; path normalization is constant-time and only used in test harness setup.

## bd-1un · TEST-E2E-PTY-CI: enforce PTY capability preflight in CI env docs/check

1. Coupling: Interactive TUI E2E in CI is now coupled to explicit PTY preflight (`scripts/e2e/pty_preflight.sh`) and associated log paths in `full-confidence` artifacts.
2. Untested claims: This environment intentionally fails PTY preflight (permission denied), so successful path is validated by script logic and CI wiring but requires first remote CI run confirmation.
3. Nondeterminism: No product-path nondeterminism introduced; preflight is a deterministic capability probe with explicit pass/fail report.
4. Security and privacy: Preflight logs include only capability diagnostics (no sensitive payloads), improving visibility without adding secret-bearing outputs.
5. Performance cliffs: Minimal overhead (single `script -qefc true` probe) relative to interactive TUI test runtime.

## bd-10s · FRESH-EYE-AUDIT: random deep code traversal and launch-doc wording pass

1. Coupling: Export manifest generation is now explicitly coupled to min/max commit index semantics instead of first/last ordering assumptions, which improves robustness for externally produced or malformed event orderings.
2. Untested claims: We validated the unordered commit-index range path with a focused regression test, but we did not add a broader malformed-eventlog recovery suite in this bead.
3. Nondeterminism: No new nondeterminism introduced. The export range computation is pure and deterministic, and launch-doc wording edits do not affect runtime behavior.
4. Security and privacy: No new secret surfaces introduced. The code change is metadata-range logic only; doc changes are wording and scope clarifications.
5. Performance cliffs: No material performance impact; the range fold is linear over already loaded events and replaces an O(1) first/last read with an O(n) scan, which is acceptable at current v0.1 scale and export path frequency.

## bd-1iv · UBS-HARDEN: production-path panic/unwrap cleanup and parser-context improvements

1. Coupling: UBS hygiene now has tighter coupling to test-style patterns in `crates/panopticon-export/src/lib.rs`; future test authors should prefer `matches!` assertions over panic branches to keep scanner noise controlled.
2. Untested claims: This bead focused on UBS signal quality and did not claim new runtime behavior; production export/refusal behavior remained covered by existing unit and integration suites.
3. Nondeterminism: No new nondeterminism introduced; edits were test-logic refactors only, with deterministic assertions and no ordering/time/randomness changes.
4. Security and privacy: Secret-scanner fixtures were rewritten to avoid obvious literal credential forms while preserving refusal-path coverage, reducing accidental “hardcoded secret” noise without weakening checks.
5. Performance cliffs: No product-path performance impact; only test code changed and full quality gates remained green.

## bd-3fw · ROBOT-MODE-CLI-V1: agent-optimized deterministic CLI contract

1. Coupling: CLI behavior now couples to an explicit output-mode contract (`--json`, `--human`, non-TTY auto-JSON) and structured error envelope fields; downstream automation should treat those keys as the stable interface.
2. Untested claims: We validated output-mode selection, quick-help compactness, and normalization helpers with unit tests, but did not yet add full integration snapshots for every command/error combination in JSON mode.
3. Nondeterminism: No truth-path nondeterminism introduced; robot-mode output is deterministic for identical inputs, and intent-repair is limited to unambiguous alias/flag normalization.
4. Security and privacy: Structured errors can now surface path strings and scanner refusal metadata in JSON output, which improves agent triage but should still be treated as operational output not intended for secret-bearing logs.
5. Performance cliffs: JSON envelope serialization adds negligible overhead compared with command execution; the main future risk is contract bloat increasing token usage if response fields grow without discipline.

## bd-1rl · TEST-E2E-PTY-FLAKE-HARDEN: stabilize interactive transcript assertions

1. Coupling: PTY interactive retry behavior is now coupled to transcript marker validation, not only process exit status, which makes capture expectations explicit in test harness logic.
2. Untested claims: We validated the retry-on-missing-marker path in normal runs, but we did not add a synthetic fault-injection harness to force partial transcript captures deterministically.
3. Nondeterminism: Product-path determinism is unchanged; this change only hardens test harness handling of environmental PTY capture jitter.
4. Security and privacy: Additional assertion logs can include transcript file paths and stderr context; they do not introduce new secret-bearing surfaces.
5. Performance cliffs: Test runtime impact is bounded (single extra retry at most) and only applies to PTY interactive tests when first attempt output is incomplete.

## bd-24k · ROBOT-MODE-CLI-V1.1: strict contract guarantees

1. Coupling: Robot-mode consumers are now coupled to an explicit envelope schema version (`panopticon-cli-robot-v1.1`) and required key set, reducing ambiguity but requiring version-aware clients for future schema evolution.
2. Untested claims: We added integration coverage for no-arg auto-JSON, invalid args, and not-found failures; we did not yet add golden snapshot tests for every success payload variant across all subcommands.
3. Nondeterminism: No truth-path nondeterminism introduced; contract changes are output-shape and parsing behavior only, and remain deterministic for identical inputs.
4. Security and privacy: Structured envelopes surface actionable path-level diagnostics and suggestions; this improves automation but means logs should still be treated as operational artifacts.
5. Performance cliffs: Envelope standardization adds negligible serialization overhead; the main long-term risk is payload growth if optional fields are added without token-budget discipline.

## bd-2fp.5 · A1-4: Pipeline pass-reduction spike (profile-gated)

1. Coupling: `panopticon-tour` now depends on `EventLogWriter::append` result accessors for committed-sequence capture; this is an explicit cross-crate contract between Tour and core write-path APIs.
2. Untested claims: We validated append-sequence/readback equivalence with a focused unit test, but we did not add a large-fixture equivalence test that compares old-vs-new pipeline internals under stress-size inputs.
3. Nondeterminism: No new nondeterminism introduced; committed ordering still flows from append-writer `commit_index`, and deterministic artifact/hash checks remain green.
4. Security and privacy: No new secret-handling surfaces were introduced; change is in Tour pipeline event flow only.
5. Performance cliffs: Pass reduction removes one full EventLog reread in Tour, but measured gains may be modest when fsync/write dominates runtime; avoid extrapolating this as a universal latency win without privileged profiler evidence.

## bd-1z3.1 · COV-1: reproducible coverage baseline capture

1. Coupling: Coverage planning now depends on the refreshed matrix in `docs/testing/coverage-matrix-v0.1.md`; follow-on beads should update this file to avoid stale risk assumptions.
2. Untested claims: Numeric line/branch percentages remain unavailable until `cargo-llvm-cov` is installed; this pass relies on test inventory counts plus risk-based gap mapping.
3. Nondeterminism: No runtime nondeterminism introduced; this bead is documentation and planning evidence only.
4. Security and privacy: No new secret-bearing data introduced; baseline references command outputs and existing local test inventory only.
5. Performance cliffs: None in product runtime; only potential process cost is maintaining matrix freshness as test suite size grows.

## bd-1z3.2 · COV-2: close high-risk uncovered paths with tests

1. Coupling: CLI contract integration tests now couple to fixture locations under `docs/assets/readme/` and `fixtures/`; moving those paths requires synchronized test updates.
2. Untested claims: We expanded success-path contract coverage for `export` and `tour`, but we still do not have exhaustive golden snapshots for every command payload permutation.
3. Nondeterminism: No runtime nondeterminism introduced; tests assert deterministic JSON envelope keys for fixed inputs.
4. Security and privacy: New tests use curated non-secret fixtures and temp output directories; no real credential material added.
5. Performance cliffs: Additional integration tests slightly increase `panopticon-tui` test runtime, but impact is modest and bounded.

## bd-1z3.3 · COV-3: e2e evidence/logging polish

1. Coupling: E2E scripts now assume robot-mode JSON output contracts (`"code":"OK"`, `"code":"EXPORT_REFUSED"`) and no-arg quick-help behavior; CLI contract changes must update script assertions.
2. Untested claims: We validated `cli_e2e.sh` and `fastlane.sh` end-to-end locally, but cross-platform shell behavior (non-bash environments) is still not part of this bead.
3. Nondeterminism: No truth-path nondeterminism introduced; logging changes add replay hints and benchmark snapshots without changing product output semantics.
4. Security and privacy: Logs gained replay hints containing command strings and local paths; still non-secret, but operators should treat artifacts as internal diagnostics.
5. Performance cliffs: Added release benchmark step in `cli_e2e.sh` increases runtime modestly; fastlane path remains budgeted and unchanged in scope.

## bd-1bv.1 · COMM-1: community health docs pass · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Public issue intake is now coupled to GitHub issue forms and security contact routing; repository rename or org migration requires updating template URLs.
2. Untested claims: We did not run live GitHub-side form rendering in this local session; validity is based on YAML structure and established template schema.
3. Nondeterminism: No product-path nondeterminism introduced. Changes are static documentation and issue-template metadata only.
4. Security: Security posture is improved by steering vuln reports to private advisory flow. Risk remains if users ignore template routing and open public issues.
5. Performance: No runtime performance impact; only repository metadata and docs were changed.

## bd-1bv.2 · COMM-2: public repo settings checklist · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Public-readiness workflow is now coupled to `docs/PUBLIC_REPO_SETTINGS_CHECKLIST.md` as the canonical operational checklist for metadata/settings flips.
2. Untested claims: Checklist recommendations (topics, branch protection, release-note shape) are governance guidance and were not validated against every GitHub org policy variant.
3. Nondeterminism: No nondeterminism introduced. This bead adds deterministic documentation only.
4. Security: Checklist encourages stronger branch/ruleset hygiene and private vulnerability flow; no new secret-bearing surfaces were added.
5. Performance: No product runtime impact. Operational overhead is limited to periodic checklist review during release prep.

## bd-qip · COMM-3: maintainer-facing PR template and submission gate guidance · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Contributor expectations are now coupled to `.github/pull_request_template.md` and `CONTRIBUTING.md`; future process changes must keep both consistent.
2. Untested claims: We did not validate GitHub web UI rendering of the PR template in this local run; correctness is based on plain markdown template semantics.
3. Nondeterminism: No product-path nondeterminism introduced; this bead is documentation/template only.
4. Security: Security reporting path is made clearer in template text, reducing accidental public disclosure risk.
5. Performance: No runtime impact.

## bd-3ou · COMM-4: add docs_guard-adjacent community health presence check · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: CI/test health now depends on a specific set of community-health files and README link references; intentional coupling to prevent accidental deletions.
2. Untested claims: Guard test does not validate semantic quality of docs, only existence and README linkage.
3. Nondeterminism: No runtime nondeterminism introduced; the new test uses deterministic file presence/string checks.
4. Security: No new secret surfaces; guard test improves governance hygiene but does not alter secret handling logic.
5. Performance: Minimal test-time overhead (single file-read pass).

## bd-zfj · COMM-5: maintainer triage and labeling playbook · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1, I2, I3, I5 (referenced as triage severity categories)
- Constitution touched: none

1. Coupling: Support flow is now coupled to `docs/COMMUNITY_TRIAGE_PLAYBOOK.md` and `SUPPORT.md` references for severity/response posture.
2. Untested claims: Recommended response windows and severity policy are governance targets, not behavior enforced by code.
3. Nondeterminism: No nondeterminism introduced; docs-only change.
4. Security: Playbook reduces risk of mishandled public security reports by explicit escalation routing.
5. Performance: No product runtime impact.

## bd-2se · COMMUNITY-NEXT: maintainer workflow and repo health automation · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none directly; supports operational trust posture around I1-I5
- Constitution touched: none

1. Coupling: Maintainer operations now rely on a small community-doc/testing framework; changes to repo policy should update templates, playbook, and guard tests together.
2. Untested claims: This feature hardens governance surfaces but does not enforce social-response timing in automation.
3. Nondeterminism: No product-path nondeterminism introduced.
4. Security: Improves policy clarity and private-report routing; no new credential or data-handling surfaces added.
5. Performance: Negligible overhead from the added guard test and docs references.

## bd-2n8 · COMM-6: fix issue-template contact links to canonical repository · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Issue-template contact links are coupled to repository origin/slug; repo transfer/rename requires updating this config.
2. Untested claims: We validated file content and local guard tests, but did not verify GitHub UI rendering in this session.
3. Nondeterminism: No runtime nondeterminism introduced; config-only update.
4. Security: Fix directly improves vulnerability-report routing by preventing misdirected links.
5. Performance: No runtime impact.

## bd-179 · COMM-7: guard canonical GitHub routing in issue template config · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Community config now encodes canonical GitHub slug in test expectations; if repository slug changes, test updates are required.
2. Untested claims: We validate file content and links statically, not live GitHub UI behavior.
3. Nondeterminism: No runtime nondeterminism introduced; deterministic file-content assertions only.
4. Security: Reduces risk of misrouted private vulnerability reports by preventing accidental link drift.
5. Performance: Negligible test overhead.

## bd-27t · COMM-8: guard README canonical GitHub slug links · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: README badge expectations now couple to canonical repository slug; repo slug changes require updating guard tests.
2. Untested claims: Guard validates static badge/release link fragments, not external URL availability.
3. Nondeterminism: No runtime nondeterminism introduced; deterministic string checks only.
4. Security: Indirectly reduces trust-surface confusion by ensuring users land on canonical CI/release endpoints.
5. Performance: Negligible test overhead.

## bd-1xi · COMM-9: public docs current-state refresh and anti-slop editorial pass · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Public guidance now couples to current CLI and community workflow semantics; future command/contract changes must update these docs promptly.
2. Untested claims: Editorial pass improves wording clarity but does not prove external platform rendering behavior.
3. Nondeterminism: No runtime nondeterminism introduced; docs-only changes.
4. Security: Security guidance is clearer and points directly to private advisory flow; no new secret-bearing surface.
5. Performance: No runtime impact.

## bd-18u · STRUCT-1: plan-only code/file reorganization audit · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Plan introduces a phased internal-module split strategy that depends on preserving public crate APIs via re-exports during refactors.
2. Untested claims: Reorganization recommendations are planning guidance only; no runtime behavior changes or proof executions were performed in this bead.
3. Nondeterminism: No runtime nondeterminism introduced. This bead adds documentation only.
4. Security: No new secret-bearing or privacy surfaces introduced.
5. Performance: No runtime impact; future phase execution should validate compile/test runtime after each split.

## bd-1de · COMM-10: lightweight public-doc local-link guard · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: none
- Constitution touched: none

1. Coupling: Public docs now couple to explicit required local links and assets through guard tests; doc/path renames require synchronized test updates.
2. Untested claims: Guard checks presence/reference only, not semantic correctness of linked content.
3. Nondeterminism: No runtime nondeterminism introduced; deterministic file existence/string checks only.
4. Security: Improves operator trust posture by reducing stale/broken public guidance risk.
5. Performance: Negligible test-time overhead from small file reads.

## bd-3n5 · STRUCT-2.1: split panopticon-export internals by concern · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I3 (share-safe export gate), I5 (loud failure), deterministic bundle/artifact expectations
- Constitution touched: none

1. Coupling: `panopticon-export` is now split across internal modules (`discover`, `secret_scan`, `bundle`) and tied via `pub(crate)` re-exports in `lib.rs`; future cross-module refactors need attention to internal visibility boundaries.
2. Untested claims: This change claims behavior-preserving module decomposition; claim is covered by full crate/unit/integration tests but not by byte-for-byte diff snapshots of every possible archive permutation.
3. Nondeterminism: No new nondeterminism sources were introduced; deterministic ordering and hashing logic were moved, not rewritten.
4. Security: Secret scanning and refusal behavior remain unchanged in logic and still fail closed; no new parsing or secret-surface expansion was introduced.
5. Performance: Runtime cost should be unchanged; only code organization changed, with negligible compile-time impact from extra modules.

## bd-d9m · STRUCT-2.2: split panopticon-tour internals by pipeline and artifact emitters · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: Tour artifact contracts (`metrics.json`, `viewmodel.hash`, `ansi.capture`, `timetravel.capture`)
- Constitution touched: none

1. Coupling: Tour flow now spans `lib.rs`, `metrics.rs`, and `artifacts.rs`; future changes must keep cross-module API alignment to avoid orchestration drift.
2. Untested claims: We claim artifact-shape preservation via refactor-only decomposition; this is covered by existing unit and integration tests but not by an external golden corpus beyond current fixtures.
3. Nondeterminism: No new nondeterministic sources introduced; hashing, sequencing, and artifact rendering logic were moved intact.
4. Security: No new input or secret-handling paths added; artifact emission and fixture parsing behavior are unchanged.
5. Performance: Runtime behavior should be equivalent; module split adds negligible compile-time overhead only.

## bd-3lr · STRUCT-2.3: split panopticon-tui CLI internals by contract and command execution · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: CLI robot-mode contract, exit-code mapping, command behavior parity
- Constitution touched: none

1. Coupling: CLI behavior is now split across `cli_contract`, `cli_normalize`, and `cli_handlers`, so changes to command fields/flags must remain synchronized across module boundaries.
2. Untested claims: We assert no behavioral drift in parsing and command execution; this is covered by existing CLI contract/unit tests but not by an external black-box snapshot for every stderr wording variant.
3. Nondeterminism: No new nondeterministic paths introduced; normalization, output-mode selection, and handler flow are deterministic and unchanged in semantics.
4. Security: No new network/process execution surface was introduced; validation and refusal paths remain fail-closed with structured errors.
5. Performance: Runtime overhead is unchanged in practice; the split mainly affects compile unit organization and maintainability.

## bd-3vv · STRUCT-2.4: core truth-path split preflight plan (plan-only) · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: canonical `commit_index` ordering, `state_hash`, `viewmodel.hash`, tour artifact contracts
- Constitution touched: referenced only (no constitutional text duplication)

1. Coupling: The preflight explicitly couples future core split sequencing to downstream crate compatibility (`import`, `export`, `tour`, `tui`) and to temporary re-export shims during migration.
2. Untested claims: This bead is plan-only; no runtime behavior changes were made or required, so claims are procedural and will be validated in future implementation beads.
3. Nondeterminism: No nondeterministic logic introduced because no truth-path code changed.
4. Security: No new security surface; plan reinforces fail-closed and deterministic validation requirements for future core work.
5. Performance: No runtime impact in this bead; performance risk is deferred and explicitly called out for future split beads.

## bd-7cy · CLI parser contract hardening matrix · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: deterministic CLI envelope semantics, bounded normalization, stable exit-code mapping
- Constitution touched: none

1. Coupling: Clap aliases now own subcommand synonym handling while `normalize_args` owns only option-shape repairs; future CLI changes must preserve this ownership boundary to avoid parser drift.
2. Untested claims: We assert no user-visible regression for canonical commands and alias forms covered by tests; untested edge cases remain for exotic shell quoting combinations outside current argv corpus.
3. Nondeterminism: No new nondeterministic sources introduced. Parsing and normalization remain deterministic and now have stricter mutation boundaries (no positional rewrites, no mutation after `--`).
4. Security: No new secret or privilege surface added. Structured error behavior remains explicit and deterministic; reduced silent rewrites lowers risk of hidden operator error in automation.
5. Performance: Runtime overhead is neutral to improved; fewer normalization branches and clap-native alias handling avoid extra heuristic passes.

## bd-1ey · CLI parser-authority durability docs guard · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: deterministic CLI contract behavior and parser-authority boundary
- Constitution touched: none

1. Coupling: AGENTS/README now explicitly couple contributor workflow to clap-as-authority and bounded normalization; future parser changes must keep docs and tests aligned.
2. Untested claims: This bead is docs/process-only; it does not itself prove runtime behavior beyond existing CLI contract tests.
3. Nondeterminism: No runtime nondeterminism introduced.
4. Security: Clarifies failure behavior and reduces risk of ambiguous parser rewrites in automation.
5. Performance: No runtime impact.

## bd-2j0 · CLI parse-error UX guidance polish · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: deterministic robot envelope schema, stable exit-code mapping
- Constitution touched: none

1. Coupling: Parse guidance now depends on clap error kinds; if clap kind semantics change, suggestions may need updates.
2. Untested claims: We cover major parse categories (invalid subcommand, missing required args, conflicts), but not every clap error kind.
3. Nondeterminism: No new nondeterminism introduced; messages/suggestions are deterministic and kind-based.
4. Security: Better guidance reduces operator mis-invocation loops; no new secret or privilege surface added.
5. Performance: Negligible runtime overhead (small match and static suggestions in parse-failure path only).

## bd-3vs · CLI argv contract corpus hardening · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: parser determinism, bounded normalization, stable machine envelope semantics
- Constitution touched: none

1. Coupling: Additional integration tests couple CLI behavior more tightly to current clap global-arg ordering semantics and alias mappings.
2. Untested claims: We still do not exhaust every argv permutation; this bead expands high-risk edge paths but does not constitute full formal proof.
3. Nondeterminism: No runtime nondeterminism introduced; tests assert deterministic behavior across edge argv topologies.
4. Security: No new security surface; better invariant coverage lowers risk of ambiguous parse handling in automation.
5. Performance: Test-only increase in CI/runtime cost is small and acceptable; production runtime unchanged.

## bd-1lh · v0.1 test completeness contract definition · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1-I5 and D1-D7 ownership mapping for test governance
- Constitution touched: none

1. Coupling: Coverage governance now depends on the coverage matrix staying aligned with crate test surfaces and CI lanes.
2. Untested claims: The contract defines 'full enough' but does not itself add runtime tests; implementation beads remain required.
3. Nondeterminism: No runtime nondeterminism introduced; docs-only change.
4. Security: Clarifies share-safe and diagnostic coverage expectations, reducing blind spots in release confidence.
5. Performance: No runtime impact; minor documentation maintenance overhead only.

## bd-1jr · CLI contract matrix gap closure · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: deterministic robot envelope schema, stable exit-code semantics, bounded normalization and replayable operator guidance
- Constitution touched: none

1. Coupling: Golden envelope assertions now intentionally couple to current CLI contract wording and keys; contract changes must update tests and docs together.
2. Untested claims: We cover high-value parse/error topologies (invalid subcommand, missing required args, unknown flag, human replay hints), but do not exhaust every clap parse-kind permutation.
3. Nondeterminism: No runtime nondeterminism introduced; tests enforce stable JSON envelope fields/messages and deterministic guidance for targeted error paths.
4. Security: No new secret or privilege surface added; improved deterministic guidance reduces operator ambiguity and accidental misuse loops.
5. Performance: Runtime unchanged in production paths; added test cases marginally increase CI test time only.

## bd-15z · PTY interactive e2e reliability and diagnostics hardening · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: deterministic test diagnostics, Truth HUD visibility checks, lens transition evidence under PTY runs
- Constitution touched: none

1. Coupling: PTY diagnostics now couple CI assertions to explicit JSON schema keys in preflight and assertion logs; schema edits must be coordinated across tests, scripts, and workflow checks.
2. Untested claims: We validate schema presence and deterministic fields via full test suite and script parse checks, but cannot assert PTY pass behavior in every CI/container runtime with restricted pseudo-terminal allocation.
3. Nondeterminism: Runtime product behavior unchanged; diagnostics now reduce ambiguity with fixed reason-code taxonomy and stable transcript-pointer fields.
4. Security: No new secret surface; logs remain local artifact outputs. Replay hints are command-only and do not expose credentials.
5. Performance: Production runtime unaffected; CI gains a small extra shell validation step and JSON log serialization overhead only in test/preflight paths.

## bd-12m · Export/Tour E2E semantic integrity expansion · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: share-safe refusal determinism, Tour artifact consistency, deterministic metadata logging
- Constitution touched: none

1. Coupling: E2E scripts now couple to artifact schema/field contracts (`metrics.json`, `timetravel.capture`, refusal report ordering); schema changes must update tests and script validators together.
2. Untested claims: We validate mixed inline+blob refusal determinism and key artifact cross-field consistency, but we still do not benchmark all scanner regex boundary variants or every importer source shape.
3. Nondeterminism: Added assertions explicitly reduce nondeterministic drift risk by requiring stable blocked-item sort order and deterministic cross-artifact hash/version linkage.
4. Security: No new external surface; stronger refusal checks improve confidence that secret-bearing exports fail closed with auditable reports.
5. Performance: Added script-side Python validations and one export integration case slightly increase CI/test time, but runtime product paths are unchanged.

## bd-3fx · CI contract enforcement for coverage diagnostics · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: coverage contract freshness, deterministic diagnostics artifacts, replayable CI failures
- Constitution touched: none

1. Coupling: CI now depends on explicit contract tags (`FL0`, `FL1`, `CC*`, `FC1`) and coverage docs/headings; doc structure changes require synchronized CI/script updates.
2. Untested claims: We validate contract checker and CI assertions locally, but cross-runner timing variance for GitHub-hosted environments is only covered by workflow execution, not local simulation.
3. Nondeterminism: New checks reduce nondeterminism in failure analysis by requiring deterministic stage markers and explicit replay commands in logs.
4. Security: No new runtime secret surface; stronger refusal/report stage checks increase confidence that share-safe guarantees remain enforced in CI.
5. Performance: Added CI checks and artifact uploads increase pipeline cost slightly; sequencing remains budget-aware by keeping heavy validation in full-confidence and compact checks in fastlane.

## bd-219 · TEST-4.6 coverage initiative closeout · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1-I5 and D1-D7 coverage ownership/completeness posture
- Constitution touched: none

1. Coupling: Coverage closeout now depends on alignment between `coverage-matrix-v0.1.md`, `FASTLANE.md`, CI contract checks, and bead status; drift in one surface can reduce operator trust in the whole matrix.
2. Untested claims: Closeout validates governance and deterministic diagnostics posture, but does not claim exhaustive proof over every CLI permutation or malformed payload family.
3. Nondeterminism: No new runtime nondeterminism introduced; closeout explicitly reinforces deterministic stage markers and replayable failure evidence expectations.
4. Security: No new runtime secret surface added; stronger refusal/export and diagnostics checks improve confidence that share-safe behavior fails closed with actionable evidence.
5. Performance: No production runtime impact; testing/CI overhead remains slightly elevated due to additional contract checks and artifact verification.

## bd-qra · TEST-4.7 capability-gated PTY CI and flake-budget enforcement · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (determinism is testable in CI), I5 (loud failure posture), PTY diagnostics contract stability
- Constitution touched: none

1. Coupling: CI now couples PTY behavior to structured preflight schema + assertion log schema; schema drift requires synchronized updates across tests, scripts, and workflow checks.
2. Untested claims: We enforce capability-gated semantics and flake budgets in automation, but cannot guarantee PTY availability parity across every hosted runner image revision.
3. Nondeterminism: This reduces nondeterministic CI failures by gating PTY-only checks behind explicit capability checks while preserving deterministic contract markers in logs.
4. Security: No new runtime secret surface; replay hints remain command-only and diagnostic artifacts are local CI outputs.
5. Performance: Minor CI overhead increase from additional contract validation; interactive PTY runs are skipped on unsupported runners, reducing wasted retry cycles.

## bd-22i · TEST-4.8 fastlane PTY capability-gating contract · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (determinism in CI), I5 (loud failure posture), deterministic stage telemetry in PR smoke lane
- Constitution touched: none

1. Coupling: Fastlane now couples to PTY preflight artifact shape and stage names (`tui_pty_preflight`, `tui_interactive_smoke`); contract changes require synchronized script/CI/doc updates.
2. Untested claims: We validated local pass/fail PTY gating paths and stage emissions, but hosted runner image churn can still alter PTY availability rates.
3. Nondeterminism: This reduces nondeterministic PR failures by converting PTY capability variance into explicit, deterministic gating metadata instead of implicit test behavior.
4. Security: No new security surface; logs remain local CI artifacts and replay guidance is command-only.
5. Performance: Slight additional script overhead for preflight parsing is negligible; skipping interactive PTY smoke on unsupported runners avoids wasted cycles.

## bd-2gs · TEST-4.9 clarify PTY flake-checker lane scope diagnostics · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (deterministic CI diagnostics), operator replayability expectations
- Constitution touched: none

1. Coupling: Checker diagnostics now couple to the fastlane run-id marker (`fastlane-v0.1`) for wrong-lane detection.
2. Untested claims: We tested lane-scope diagnostics against local fastlane/full-confidence-style outputs; hosted CI path correctness still depends on preserving run-id contracts.
3. Nondeterminism: No runtime nondeterminism added; this change improves deterministic error interpretation for manual operator invocations.
4. Security: No new secret surface introduced; messaging changes only.
5. Performance: Negligible script overhead (single `rg` check on run log).

## bd-qhk · A2-1 baseline refresh pack for representative workloads · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (determinism evidence in CI/proof loops)
- Constitution touched: none

1. Coupling: Baseline evidence now couples optimization decisions to current fixture/workload mix (`fastlane`, `cli_e2e`, `tour --stress`, `bench_tour`).
2. Untested claims: Baseline is local-environment evidence; it does not independently prove cross-host equivalence of wall-clock numbers.
3. Nondeterminism: No runtime behavior changes introduced; this bead records measurements only.
4. Security: No new secret-bearing data introduced; artifacts contain timing/resource metadata and existing deterministic evidence paths.
5. Performance: No product runtime impact; enables tighter optimization prioritization by replacing stale assumptions with fresh measurements.

## bd-2jw · A2-2 hotspot profiling evidence (CPU/allocation/I-O) · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (deterministic evidence and replayable diagnostics)
- Constitution touched: none

1. Coupling: Added `profile_tour` stage-profile path couples optimization planning to current Tour pipeline stage boundaries.
2. Untested claims: Fallback profile reports stage wall-time percentages, not kernel sampled stacks; deeper symbol-level attribution still needs privileged host profiling.
3. Nondeterminism: Runtime behavior for normal commands unchanged; profiling output is deterministic for a fixed fixture and iteration count.
4. Security: No new secret surface; outputs include only timing/resource metadata and existing artifact paths.
5. Performance: No production-path regression expected; profiling helper is opt-in and used only for measurement workflows.

## bd-14f · A2-3 opportunity matrix and candidate selection · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I2 (deterministic projection), I4 (testable determinism)
- Constitution touched: none

1. Coupling: Candidate ranking is now coupled to current fixture/workload evidence and stage-profile methodology; future workload shifts require matrix refresh.
2. Untested claims: Matrix includes proof and test plans but does not itself execute the selected optimization; empirical gain remains to be validated in `bd-qx4`.
3. Nondeterminism: No runtime behavior changes introduced; this bead is planning/evidence documentation only.
4. Security: No new security surface; explicit rollback/guardrail definitions reduce risk of unsafe optimization rollout.
5. Performance: No direct runtime impact; narrows implementation to one high-value lever to avoid attribution noise.

## bd-qx4 · A2-5 implement one-lever performance optimization (C1 in-place reducer) · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1 (truth ordering), I2 (deterministic projection), I4 (determinism testability)
- Constitution touched: none

1. Coupling: Replay-heavy paths now share mutable reducer transition logic; accidental divergence between `reduce` and `reduce_in_place` would be a future maintenance risk.
2. Untested claims: Improvement evidence is currently fixture-focused (`large-stress`); additional production-shaped inputs may still shift stage distribution.
3. Nondeterminism: Transition semantics remain deterministic; no new randomness/time ordering dependencies introduced.
4. Security: No new secret or trust-boundary surface introduced; this is internal state-transition performance work.
5. Performance: Large observed gains in current baseline; new dominant stages are parse/append, which become next optimization targets if needed.

## bd-18m · A2-4 investigation-flow UX audit (desktop + narrow) · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (deterministic evidence); D5 (Incident default / Forensic toggle)
- Constitution touched: none

1. Coupling: UX conclusions are tied to current deterministic fixture captures and may require refresh when layout/content density evolves.
2. Untested claims: Audit identifies friction and proposes a narrow-safe action-hint fix; it does not itself validate post-fix operator time-to-answer metrics.
3. Nondeterminism: No runtime behavior change; audit only uses deterministic artifacts and scripted logs.
4. Security: No new security surface introduced; refusal-path review reinforces operator clarity around blocked-secret recovery flow.
5. Performance: No runtime impact; improved guidance is expected to reduce operator interaction overhead, not compute cost.

## bd-hov · A2-6 narrow-safe incident guidance UX improvement · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: D5 (Incident default and Forensic navigation), I4 (deterministic evidence)
- Constitution touched: none

1. Coupling: Incident Lens layout now couples anomalies section height to width-aware hint wrapping, so future copy changes should preserve wrap-aware budgeting.
2. Untested claims: We validated narrow marker visibility and modality contracts, but did not yet measure user task-time reduction with human usability trials.
3. Nondeterminism: No truth-path changes; rendering remains deterministic for same state and dimensions.
4. Security: No new security surface introduced; this is presentation-only guidance refinement.
5. Performance: Minimal rendering overhead (small string/line calculations); no meaningful compute or memory risk.

## bd-3q5 · A2-7 enterprise-ready closeout report · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1-I5/D1-D7 reporting posture (no behavioral changes)
- Constitution touched: none

1. Coupling: Closeout metrics are currently anchored to Tour profile workload and fixture shape; downstream decisions should refresh evidence when workload assumptions change.
2. Untested claims: Report summarizes deterministic and UX test evidence but does not claim exhaustive real-user usability outcomes or production-host perf parity.
3. Nondeterminism: No runtime behavior changes introduced; report-only aggregation.
4. Security: No new security surface; report highlights refusal safety and deterministic contract posture without exposing secrets.
5. Performance: No runtime impact; documentation improves decision quality for subsequent optimization rounds.

## bd-6wkf · A3-1 stream fixture parsing with equivalence proof · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4 (ordering and determinism)
- Constitution touched: none

1. Coupling: Tour parse stage now couples directly to file-stream semantics; malformed I/O edge behavior remains delegated to existing parser behavior.
2. Untested claims: We proved event-sequence equivalence for representative fixture mode; broader malformed corpus parity is still covered primarily by importer tests.
3. Nondeterminism: No new nondeterminism introduced; parsing order remains source-line order.
4. Security: No new secret surface; change only affects read-path buffering strategy.
5. Performance: Expected memory improvement from avoiding full-file string buffering; actual hotspot/memory deltas are measured in A3-2.

## bd-2aum · A3-2 post-C2 hotspot and memory profile refresh · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I4 (determinism evidence)
- Constitution touched: none

1. Coupling: Profiling conclusions are tied to current representative fixture and profile_tour staging model.
2. Untested claims: Measurements indicate improvement, but no cross-host statistical confidence intervals were collected.
3. Nondeterminism: No runtime behavior changes introduced; evidence-only bead.
4. Security: No new security surface; profiling outputs contain only performance counters and deterministic stage metrics.
5. Performance: C2 appears beneficial; next optimization decision should avoid overfitting to single workload signature.

## bd-3ufi · A3-3 C3 decision gate · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4 (avoid risky optimization without clear ROI)
- Constitution touched: none

1. Coupling: Decision policy now explicitly ties C3 execution to hotspot dominance and proof-cost thresholds.
2. Untested claims: No implementation changes were made; only decision rationale captured from current evidence.
3. Nondeterminism: No runtime behavior changes; no new nondeterminism risk introduced.
4. Security: No new security surface; no code-path changes in append writer.
5. Performance: Potential append-path gains deferred; chosen to avoid complexity without clear dominant-hotspot justification.

## bd-3vv0 · A3-4 C3 implementation bead closure (skipped) · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Result: closed without code changes per A3 decision gate

1. Coupling: Avoided adding new append durability-mode coupling in this round.
2. Untested claims: C3 behavior remains unimplemented in A3; no new claims introduced.
3. Nondeterminism: No changes; existing deterministic append semantics remain intact.
4. Security: No changes; no new surface introduced.
5. Performance: Potential append-path optimization deferred; parse remains next likely target.

## bd-141p · A3-5 enterprise closeout report · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: reporting only; no behavior changes
- Constitution touched: none

1. Coupling: Closeout numbers are tied to current fixture and profile command settings.
2. Untested claims: Report summarizes measured outcomes; it does not establish broad production-SLA guarantees across all environments.
3. Nondeterminism: No runtime changes; documentation-only aggregation.
4. Security: No new surface; report references deterministic and refusal-safe behavior without exposing secrets.
5. Performance: No runtime impact; improves prioritization clarity for next optimization round.

## bd-xx6w · A4 typed cassette parser path optimization · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4
- Constitution touched: none

1. Coupling: Importer mapping now depends on a typed record schema instead of ad-hoc JSON field access; future cassette format expansion needs schema updates.
2. Untested claims: Existing test suite covers current mapped fields well, but uncommon unknown-field edge patterns still rely on generic fallbacks.
3. Nondeterminism: No new nondeterminism introduced; parser remains line-order deterministic.
4. Security: No new secret surface; no changes to refusal logic or export pathways.
5. Performance: Evidence shows parse-share reduction and p95 improvement; occasional p99 variance remains and should be monitored in future rounds.

## bd-qhgs · A4-2 zero-allocation fractional timestamp parsing · 2026-02-17

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4
- Constitution touched: none

1. Coupling: Timestamp parsing logic is now more explicit and low-level; future format expansion must keep truncate/pad semantics in sync with historical importer behavior.
2. Untested claims: Current tests cover key fractional edge cases; broader malformed timestamp families still rely on existing parser fallback behavior.
3. Nondeterminism: No new nondeterminism introduced; parsing remains pure and source-order deterministic.
4. Security: No new trust-boundary or secret-handling surface introduced.
5. Performance: Small but measurable parse-share reduction observed; append remains dominant hotspot for future rounds.

## bd-2sep · DOCS-1 launch/checklist reconciliation · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: documentation/process only; no truth-path behavior change
- Constitution touched: none

1. Coupling: README and verification docs now couple more tightly to current CLI help-mode behavior; future CLI contract changes must update both docs together.
2. Untested claims: GitHub UI settings (description/topics/homepage/branch protection/actions policy) still require manual confirmation outside local workspace.
3. Nondeterminism: No runtime nondeterminism introduced; this bead only updates docs and checklist state.
4. Security: Reduced risk of accidental public leakage by replacing strategy-style wording with neutral release-language in launch planning docs.
5. Performance cliffs: No runtime impact; process overhead is limited to periodic manual checklist reconciliation before public flips/releases.

## bd-1wc1 · DOCS-2 full audit matrix for planning/checklist docs · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: docs/process only
- Constitution touched: none

1. Coupling: Audit matrix now couples release-readiness review to a single summary artifact (`docs/DOCS_AUDIT_MATRIX_2026-02-18.md`); future doc-set changes should refresh this matrix.
2. Untested claims: Manual GitHub UI settings cannot be proven from local repo state and remain explicitly marked as manual pending.
3. Nondeterminism: No runtime behavior changes; documentation-only update.
4. Security: Clarifies what remains platform-side and prevents accidental over-claiming of public-readiness state.
5. Performance cliffs: None; no code-path or runtime changes introduced.

## bd-3gxs · SHOWCASE-1 detailed research for showcase UX profile · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: docs/research only; no truth-path code changes
- Constitution touched: none

1. Coupling: Showcase profile direction now couples launch media and capture assets to a named UI profile contract (`showcase`) that must remain presentation-only.
2. Untested claims: Research recommends style/layout changes and static showcase page strategy; implementation and no-drift proof tests are still required in follow-up beads.
3. Nondeterminism: No runtime changes in this bead; risk lies in future implementation if visual effects accidentally alter deterministic capture pathways.
4. Security: No new security surface from research itself; future showcase page must avoid introducing unverifiable marketing claims.
5. Performance cliffs: Potential extra render overhead in showcase mode is currently unmeasured and should be bounded during implementation.

## bd-1siy · SHOWCASE-2 implement showcase UI profile in panopticon-tui · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4 (presentation-only changes; no truth-path mutation)
- Constitution touched: none

1. Coupling: Lens and HUD rendering now depend on `UiProfile`; future visual variants must preserve default `Standard` behavior and avoid branching in reducer/projection code paths.
2. Untested claims: Current tests validate CLI parsing, render compatibility, and full suite health; explicit screenshot-diff assertions for showcase captures remain a follow-up if we want strict visual regression guardrails.
3. Nondeterminism: No new nondeterminism introduced; profile selection changes styles/border types only and keeps event ordering/hash computation untouched.
4. Security: No secret-handling or export-safety logic changed; README/demo updates remain command-accurate and verifiable.
5. Performance cliffs: Showcase profile introduces additional style modifiers and rounded borders, with expected negligible overhead; if expanded with heavier effects later, capture and tour paths should be benchmarked.

## bd-233m · SHOWCASE-3 deterministic screenshot artifacts from README captures · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I2/I4 (presentation artifacts only)
- Constitution touched: none

1. Coupling: README visuals now depend on the capture generator’s SVG renderer; visual style updates should be centralized in that renderer to avoid drift.
2. Untested claims: SVG output format is tested for escaping and presence, but no pixel-diff snapshot testing is in place yet.
3. Nondeterminism: SVG generation is deterministic from deterministic text captures; no time/random inputs are used in rendering.
4. Security: SVG text is XML-escaped to avoid markup injection from fixture content.
5. Performance cliffs: Asset generation does extra file writes; impact is limited to docs asset refresh, not runtime hot paths.

## bd-2kni · SHOWCASE-4 presentation showcase page from deterministic assets · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: docs-only; no truth-path changes
- Constitution touched: none

1. Coupling: Showcase page currently references local asset paths under `docs/assets/readme`; path changes require synchronized updates.
2. Untested claims: Page content is static and command references are validated manually; no dedicated link-check job yet for the new docs page.
3. Nondeterminism: None introduced; docs and generated assets are deterministic artifacts.
4. Security: No secret surface added; page only references fixture-based captures and deterministic commands.
5. Performance cliffs: None in runtime; minor docs footprint increase only.

## bd-3ly8 · PRODUCT-1 foundational product and GTM docs · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: docs/product strategy only; no runtime behavior changes
- Constitution touched: none

1. Coupling: Product docs now establish a baseline messaging system; future launch copy should stay aligned with these files to avoid narrative drift.
2. Untested claims: GTM and business model assumptions are strategic hypotheses; they require market validation post-launch.
3. Nondeterminism: No runtime changes introduced.
4. Security: No additional secret or trust boundary surface; docs avoid hardcoded credentials and unverifiable claims.
5. Performance cliffs: None; documentation-only change.

## bd-224u · PRODUCT-2 founder one-pager and 30/60/90 roadmap · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: strategy/docs only
- Constitution touched: none

1. Coupling: Roadmap priorities now reference current showcase and proof posture; major product direction changes should update these docs to avoid stale planning.
2. Untested claims: Market and conversion assumptions are directional and require validation through launch and user interviews.
3. Nondeterminism: No runtime changes introduced.
4. Security: No new secret surface or sensitive operational detail exposure.
5. Performance cliffs: None; documentation-only updates.

## bd-1kt1 · SHOWCASE-5 three-demo track plan · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: planning/docs only
- Constitution touched: none

1. Coupling: Demo plan now defines sequence and naming for showcase tracks; follow-up implementation beads should preserve these IDs for traceability.
2. Untested claims: The plan outlines implementation intent; no runtime behavior or measurable outcomes changed in this bead.
3. Nondeterminism: No runtime changes introduced.
4. Security: No new data-handling surface; plan explicitly preserves share-safe and truth-path guardrails.
5. Performance cliffs: None; docs-only planning artifact.

## bd-q1la · SHOWCASE-6 Determinism Duel demo script and docs wiring · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4
- Constitution touched: none

1. Coupling: Demo script depends on tour CLI flags and artifact paths; future CLI contract changes must keep script and docs in sync.
2. Untested claims: Script is validated manually and by full test suite context, but lacks a dedicated automated shell test.
3. Nondeterminism: The script explicitly checks deterministic replay via hash equality and fails loudly on mismatch.
4. Security: Uses local fixture data only and does not introduce new secret-handling surfaces.
5. Performance cliffs: Runs two stress tours; intended for demo/proof workflows, not tight inner-loop development usage.

## bd-1ggs · SHOWCASE-7 fast/full modes + demo2/demo3 + CI smoke · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: I1/I2/I4
- Constitution touched: none

1. Coupling: Demo scripts and CI now rely on stable CLI commands and fixture paths; command or path changes must update scripts and docs together.
2. Untested claims: Full-mode CI execution is intentionally not default due to runtime cost; full paths remain manually validated.
3. Nondeterminism: Fast smoke path checks deterministic hash equality and refusal/report behavior; full mode remains available for stress-grade validation.
4. Security: Refusal demo uses fixture-generated refusal reports and does not expose external secrets; outputs remain local.
5. Performance cliffs: Full demo modes are intentionally heavier and should be used for showcase/proof runs, while fast mode is optimized for CI and quick checks.

## bd-rqeg · Release readiness closure pass (checklist + waiver review) · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: docs/process only
- Constitution touched: none

1. Coupling: Release-readiness reporting now depends on current checklist/doc paths; future checklist renames should update `docs/RELEASE_READINESS_2026-02-18.md`.
2. Untested claims: GitHub UI/admin settings remain manually verified items and cannot be fully asserted from local repo state.
3. Nondeterminism: No runtime or projection logic changed; docs and waiver metadata only.
4. Security: No new data-handling surface; waiver text avoids secrets and records only tool availability facts.
5. Performance cliffs: None; this pass adds no runtime work and no new hot-path behavior.

## bd-im6j · Numeric coverage enablement and waiver retirement · 2026-02-18

Context:
- Bead owner: GreenEagle (codex-cli)
- Invariants referenced: process/testing only
- Constitution touched: none

1. Coupling: Full-confidence CI now depends on `taiki-e/install-action@cargo-llvm-cov` and `scripts/testing/coverage_numeric.sh`; future CI platform changes must preserve this toolchain path.
2. Untested claims: Local environments without llvm-cov still cannot produce numeric reports; this is now acceptable because release gating is CI-backed, not local-only.
3. Nondeterminism: Coverage outputs are informational artifacts and do not feed truth-path hashing; no reducer/projection ordering behavior changed.
4. Security: Coverage artifacts can expose source file path structure; they should remain in CI artifacts and not be treated as secret-free export payloads by default.
5. Performance cliffs: Coverage jobs are heavier than regular tests; scoped to full-confidence lane to avoid slowing fastlane/PR iteration.

## bd-2ykd · A1: closure-evidence parity audit + taxonomy · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: process-only governance hardening
- Constitution touched: none

1. Coupling: New audit output couples `docs/RISK_REGISTER.md` heading conventions with `.beads/issues.jsonl` closure state; future heading format changes require synchronized audit script updates.
2. Untested claims: Classification heuristics (milestone alias and parent rollup coverage) are policy choices, not runtime truths; follow-on CI guard (`bd-110f`) must codify exact exemption semantics.
3. Nondeterminism: Audit classification is deterministic for a fixed repo state (sorted IDs, stable parsing); no truth-path or projection behavior changed.
4. Security: Audit artifacts include issue titles and close reasons; they contain no secrets by design but should still be treated as internal governance outputs.
5. Performance cliffs: Negligible runtime cost; script executes quickly and scales linearly with issue count.

## bd-2pyi · B1: adapter contract framework and normalization boundaries · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: `panopticon-import` now centralizes adapter contract behavior in `contract.rs`; new adapters must conform to shared helpers (schema-version validation and commit-index rejection).
2. Untested claims: Compatibility with future provider schemas is still prospective; each new adapter must add fixture-backed contract tests before relying on this framework.
3. Nondeterminism: No new nondeterministic behavior introduced; normalization and contract checks are pure and deterministic over input records.
4. Security: Rejecting source-supplied `commit_index` tightens trust boundaries and reduces risk of forged canonical ordering metadata.
5. Performance cliffs: Contract checks add minimal per-record overhead (string/option checks) and are unlikely to affect importer throughput materially.

## bd-2ut2 · B2: OpenAI Responses adapter v1 · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: The adapter currently maps a focused OpenAI Responses subset (`response.*` and function-call items); future upstream event families should extend `map_payload` in one place to preserve predictable semantics.
2. Untested claims: We do not yet validate every historical/legacy OpenAI Responses variant; this is addressed by upcoming cross-adapter conformance corpus work.
3. Nondeterminism: Parser preserves source line order, uses stable fallbacks for IDs, and emits deterministic payload maps (`BTreeMap`), so no new ordering nondeterminism is introduced.
4. Security: Source-provided `commit_index` is rejected and schema mismatches are surfaced as contract errors, reducing risk of untrusted canonical-order injection.
5. Performance cliffs: Parsing is line-by-line and non-stream-buffered beyond input reader semantics; very large `item` payloads may increase allocation overhead, mitigated by existing payload-ref/blob boundaries in downstream append path.

## bd-cpa2 · B3: Anthropic messages/tool-use adapter v1 · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: Anthropic adapter shares normalization and contract helpers with other importers; schema/version upgrades should continue through shared `contract.rs` to avoid divergent trust rules.
2. Untested claims: The adapter currently targets common message/tool-use envelopes; additional Anthropic variants may need fixture expansion in B5 conformance work.
3. Nondeterminism: Output ordering is source-line driven, IDs have deterministic fallbacks, and generic payload maps use `BTreeMap`, so no new nondeterministic iteration is introduced.
4. Security: Source-supplied `commit_index` is rejected and schema drift is converted to explicit contract errors, preserving append-writer ownership of canonical order.
5. Performance cliffs: Tool candidate extraction traverses a small list of possible fields per record; this remains bounded and linear per line for current schemas.

## bd-14ja · B4: Cohere Translate adapter v1 (vertical differentiator) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: Cohere Translate mapping introduces a domain-specific adapter contract that still depends on shared normalization helpers; schema updates should remain centralized in `contract.rs`.
2. Untested claims: Only a minimal v1 event surface is covered today; broader production variants (batch modes, retries, nested policy metadata) should be added in B5 conformance corpus.
3. Nondeterminism: Parser is line-ordered, uses deterministic fallback IDs, and avoids unordered containers in payload data, so replay ordering and serialization remain stable.
4. Security: Contract checks reject source-owned `commit_index` and schema mismatch is surfaced as explicit contract errors, preserving trust boundary between importer input and canonical ordering.
5. Performance cliffs: Request argument synthesis includes source length metadata; very large source texts increase per-record string processing but remain linear and bounded by input line size.

## bd-eoat · B5: cross-adapter replay conformance corpus + deterministic drift gate · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, I4, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: CI now depends on adapter drift script and test names; if test target or script path changes, workflow steps must be updated together.
2. Untested claims: The conformance corpus currently covers representative small/noisy fixtures; larger and provider-edge corpora should be expanded in future perf/governance tracks.
3. Nondeterminism: Drift gate reruns the same fixtures repeatedly and asserts byte-equal serialized ImportEvent outputs, directly detecting ordering/serialization instability.
4. Security: Corpus fixtures are synthetic and local; no secrets are introduced, and drift gate validates importer contract boundary (no source commit_index ownership).
5. Performance cliffs: CI adds an extra adapter-conformance test step; runtime cost is low now but may grow with corpus size and should remain bounded.

## bd-dvmi · C1: deterministic run-delta engine (causal diff core) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, I4, D6 ownership boundary
- Constitution touched: none

1. Coupling: `panopticon-core::delta` now depends on the committed-event schema shape; adding/removing committed fields or payload serialization changes will require coordinated delta updates.
2. Untested claims: We validate deterministic output and key divergence classes, but do not yet cover very large run-pair diffs for memory profiling; that belongs in the next compare-command bead integration path.
3. Nondeterminism: No new nondeterminism introduced. Diff traversal is keyed by `BTreeMap/BTreeSet`, paths are sorted deterministically, and matching is by canonical `commit_index` only.
4. Security: No new secret-handling surface was added; this bead computes metadata-only divergence records from already-ingested committed events.
5. Performance cliffs: Current implementation flattens payload JSON for both sides per matching index, which is linear in payload size and could become expensive on very large payloads; acceptable for v0.1 and now isolated for future targeted optimization.

## bd-dvmi · C1 post-close fresh-eye corrections (run-id/payload_ref determinism) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, I4, D6 ownership boundary
- Constitution touched: none

1. Coupling: Delta summary metadata now derives run IDs from canonical lowest `commit_index` event, coupling summary semantics to canonical ordering rather than input slice order.
2. Untested claims: We still assume committed streams do not intentionally mix multiple run IDs under one comparison side; if mixed, summary reflects the canonical-first run id.
3. Nondeterminism: Removed a real nondeterminism source where unsorted input could change selected `left_run_id`/`right_run_id`; payload_ref comparison now preserves None-vs-empty distinction deterministically.
4. Security: No new security surface added; change is deterministic comparison semantics only.
5. Performance cliffs: No measurable cliff introduced; comparisons remain linear in event and payload-path count.

## bd-dvmi · C1 second fresh-eye correction (duplicate commit_index tie-break determinism) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, I4, D6 ownership boundary
- Constitution touched: none

1. Coupling: Duplicate-index handling now couples to a deterministic event tie-break key derived from committed-event fields plus payload serialization.
2. Untested claims: We still treat duplicate `commit_index` inputs as recoverable comparison input, not hard errors; upstream append-writer uniqueness invariants remain the primary prevention mechanism.
3. Nondeterminism: Removed another potential nondeterminism source by making duplicate-index resolution independent of caller-provided slice order.
4. Security: No new secret surface introduced; tie-break logic runs on already-ingested committed events.
5. Performance cliffs: Tie-break key generation incurs extra serialization only when duplicate indices are present; normal unique streams are unaffected.

## bd-eymy · D1: fixed-fixture replay benchmark harness (latency + RSS) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4 (deterministic artifact contract and measurable replay behavior)
- Constitution touched: none

1. Coupling: `bench_tour` now couples to `TourMetrics.event_count_total` and fixed fixture path semantics; schema/version and fixture fields are explicit in artifact output.
2. Untested claims: Peak RSS collection is Linux-specific (`/proc/self/status`) and currently degrades to `null` on non-Linux hosts; cross-platform RSS parity remains future work.
3. Nondeterminism: Bench artifact shape is deterministic; measured latency/RSS values are intentionally environment-dependent metrics, and this is explicit in artifact provenance fields.
4. Security: Artifact contains local path and command metadata; this is expected for local/CI diagnostics and should not be treated as a public redaction-safe export artifact.
5. Performance cliffs: Running benchmark with high iteration counts can be expensive; guardrails remain via configurable `PANOPTICON_TOUR_BENCH_ITERS` and fixed fixture scope.

## bd-9jsl · C2: CLI compare command + machine contract for incident diffs · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, I4, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: `panopticon-tui` CLI now couples to `panopticon-core::delta` and `panopticon-import::cassette` parsing for compare-mode input normalization.
2. Untested claims: Compare contract tests now cover no-diff and divergence envelopes for eventlog inputs; cassette-vs-cassette compare parity is implemented but not yet covered with a dedicated fixture-pair contract test.
3. Nondeterminism: Compare loads events in source order then computes diffs by canonical `commit_index`; output envelopes and divergence payloads are serialized deterministically.
4. Security: Compare output includes local input paths and replay command suggestions; this is expected for local operator diagnostics and not a share-safe export artifact.
5. Performance cliffs: Compare flattens payload JSON for both sides during diffing, which is linear in payload size and can be expensive for very large traces; this is now isolated behind one command for targeted optimization.

## bd-28ny · C3: one-command incident evidence pack (local-first, share-safe) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I1, I2, I3, I4, I5, D6, D7 ownership constraint
- Constitution touched: none

1. Coupling: `incident-pack` now composes compare, replay/projection hashing, and export refusal checks inside `panopticon-tui`; changes in any of those contracts require coordinated pack contract updates.
2. Untested claims: Tests cover clean success and fail-closed refusal paths for eventlog inputs; mixed cassette/eventlog incident-pack permutations are implemented but not yet exhaustively covered.
3. Nondeterminism: Pack artifacts are derived from committed events in canonical order; manifest file map uses deterministic key ordering and BLAKE3 file hashes.
4. Security: Incident pack intentionally writes local file paths and normalized eventlogs to output directory for operator handoff; these artifacts are not treated as redacted public exports by default.
5. Performance cliffs: Incident pack performs replay+projection twice and executes two share-safe export scans; very large traces can incur higher CPU and I/O, but work is bounded to explicit operator invocation.

## bd-2cj9.2 · E3: strengthen contract tests for incident-pack artifact integrity · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4, I5, D7 boundary (execution fail-closed)
- Constitution touched: none

1. Coupling: CLI contract tests now couple directly to `incident-pack` artifact schema fields (`compare/delta.json`, `replay/*.json`, and manifest `files` entries). Schema changes will require coordinated test updates.
2. Untested claims: We now assert required keys/types and non-placeholder JSON objects, but we still do not inject all possible filesystem fault classes (e.g., partial write failures after directory creation).
3. Nondeterminism: No nondeterminism introduced; tests assert deterministic envelope/artifact shapes and do not alter runtime ordering/hash logic.
4. Security: New assertions inspect generated local artifacts only; no secret-handling behavior changed and refusal behavior remains fail-closed.
5. Performance cliffs: Added contract assertions slightly increase test runtime and filesystem I/O during CLI integration tests; impact is bounded and acceptable for v0.1.

## bd-2cj9.3 · E2: remove masking fallback in deterministic runtime path · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4, D7 fail-closed execution boundary
- Constitution touched: none

1. Coupling: `event_stable_tiebreak_key` now encodes serialization failures explicitly in its payload component string, coupling duplicate-index tie-break semantics to this sentinel format.
2. Untested claims: We added a direct proof test for explicit payload inclusion in tie-break keys, but we still cannot naturally trigger payload serialization failure from current `EventPayload` variants.
3. Nondeterminism: No new nondeterminism introduced; tie-break ordering remains deterministic and no longer silently collapses payload component to empty string on serialization error.
4. Security: No new secret/PII handling surface; this change only affects internal deterministic comparison keys and test harness assertions.
5. Performance cliffs: Added match-based error handling and one extra unit test; runtime cost is negligible and only on tie-break key construction path.

## bd-2cj9.4 · E4: codify CLI repair vs execution-failure contract · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I5, D7 fail-closed boundary discipline
- Constitution touched: none

1. Coupling: `AGENTS.md` and `docs/guides/CLI_DESIGN.md` now explicitly couple CLI behavior expectations to specific contract test files.
2. Untested claims: Wording alignment is tested indirectly via existing CLI contract tests, but no dedicated docs-lint enforces matrix text consistency yet.
3. Nondeterminism: No runtime nondeterminism introduced; changes are documentation policy only.
4. Security: Clarified that runtime artifact/serialize failures and export scanner findings must fail closed; this reduces ambiguity risk for future security regressions.
5. Performance cliffs: No runtime cost; only documentation maintenance overhead when contract files move or rename.

## bd-2cj9.6 · E5: runtime anti-masking guardrail check · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4, I5, D7 fail-closed execution boundary
- Constitution touched: none

1. Coupling: Added a repo guard test (`crates/panopticon-core/tests/runtime_masking_guard.rs`) that scans `panopticon-core/src` and `panopticon-tui/src` for specific serde fallback anti-patterns.
2. Untested claims: Guard currently focuses on known fallback signatures and line-window heuristics; it may miss semantically equivalent patterns expressed in very different syntax.
3. Nondeterminism: No runtime nondeterminism introduced; this is test-only scanning logic with deterministic inputs.
4. Security: Guard reduces risk of silent artifact degradation by enforcing fail-closed behavior at CI/test time; no new secret surface added.
5. Performance cliffs: Additional file-scan test adds small overhead to `cargo test`; scope is bounded to runtime source trees and fixed-window string checks.

## bd-2cj9.5 · E6: TRACK-E closeout evidence + final risk pass · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4, I5, D7
- Constitution touched: none

1. Coupling: Closeout doc now links policy wording, runtime behavior, and specific contract test names; future test renames require doc maintenance.
2. Untested claims: Closeout summarizes repeated gate runs but does not independently archive machine-readable gate artifacts in a dedicated bundle.
3. Nondeterminism: No runtime changes in this bead; documentation only.
4. Security: Clarifies fail-closed boundary and rollback constraints, reducing risk of future silent fallback reintroduction in integrity paths.
5. Performance cliffs: No runtime impact; minor documentation upkeep overhead only.

## bd-1943 · D2: perf artifact schema + trend storage contract · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4 (derived artifacts stay deterministic and out of truth path)
- Constitution touched: none

1. Coupling: `bench_tour` now owns both bench artifact schema (`panopticon-tour-bench-v1`) and trend record schema (`panopticon-perf-trend-v1`), coupling CI/reporting consumers to these versioned field sets.
2. Untested claims: Trend storage is validated with schema roundtrip tests and validator checks, but we do not yet enforce JSONL compaction/retention policy for long-running trend logs.
3. Nondeterminism: No truth-path nondeterminism introduced; new artifacts are derived diagnostics. Trend append order reflects invocation order, intentionally outside canonical EventLog semantics.
4. Security: Trend records may include optional git SHA and fixture path metadata; this is operational telemetry and not share-safe export data.
5. Performance cliffs: Trend append adds one file-open + one line write per benchmark run; overhead is negligible compared with replay benchmark execution.

## bd-1wx1 · D3: CI perf regression policy phase-1 (warn-only) · 2026-02-18

Context:
- Bead owner: ubuntu (codex-cli)
- Invariants referenced: I2, I4 (derived perf diagnostics only)
- Constitution touched: none

1. Coupling: `full_confidence` CI now depends on `bench_tour` artifact outputs and `scripts/testing/check_perf_regression_warn.sh` warning semantics.
2. Untested claims: CI workflow changes are validated via local script execution and workspace tests, but not yet exercised in a live GitHub Actions run in this session.
3. Nondeterminism: Perf metrics are inherently environment-sensitive; phase-1 is warn-only calibration to avoid false hard-fails during baseline stabilization.
4. Security: No auth or secret handling changes; perf artifact paths and optional git SHA remain CI diagnostics only.
5. Performance cliffs: `bench_tour --release` in full-confidence increases CI runtime; accepted for calibration and surfaced via non-blocking warnings.

## bd-110f · A2: enforce bead-closure evidence guard in CI · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4 (testable determinism), I5 (loud failure for governance drift)
- Constitution touched: none

1. Coupling: CI now depends on parity-audit outputs plus `docs/testing/bead-closure-evidence-exemptions-v0.1.json`. This coupling is intentional and auditable; it keeps closure evidence policy executable instead of narrative-only.
2. Untested claims: The guard script is validated with positive and negative local runs, but there is no separate unit-test harness for Python scripts in this repo. CI execution is the primary regression check for script behavior.
3. Nondeterminism: Guard output is deterministic for a fixed tracker/risk state; IDs are sorted and replay commands are static. The parity report timestamp remains fixed by the underlying audit script (`generated_at` constant), so no wall-clock instability is introduced by this bead.
4. Security: No secrets or PII introduced. The exemption ledger contains bead IDs and rationale only. Main risk is process misuse (over-broad exemptions), mitigated by explicit ID-based entries and stale-exemption warnings.
5. Performance: Added guard is lightweight (single JSONL/markdown scan + JSON parse) and runs quickly in fastlane/full-confidence. No meaningful runtime cliff expected.

## bd-25a0 · A3: reconcile historical closure gaps and backfill evidence · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4 (testable governance evidence), I5 (loud failure posture for process drift)
- Constitution touched: none

1. Coupling: Historical closure parity is now explicitly coupled to the temporary exemption ledger. This is intentional for auditability, but it creates maintenance pressure to prune exemptions as exact entries are backfilled.
2. Untested claims: We assert reconciliation correctness through guard output (`unresolved=0`) and generated audit artifacts; there is still no standalone Python unit-test harness for governance scripts.
3. Nondeterminism: No runtime nondeterminism added; reconciliation artifacts are deterministically generated from current tracker/risk state. The underlying parity report keeps a fixed generated timestamp, avoiding wall-clock drift in output shape.
4. Security: No new data exposure. Risk is governance misuse if exemptions become permanent; mitigated by explicit rationale entries and stale-exemption warnings.
5. Performance: Reconciliation step is lightweight and CI-safe. Main cliff would be unbounded growth of historical exemptions, addressed by future pruning/backfill work.

## bd-1idz · A4: normalize script execution ergonomics for test-governance scripts · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4 (testability and repeatability of governance checks)
- Constitution touched: none

1. Coupling: CI and docs now assume governance scripts are executable and invoked directly via shebang (`scripts/testing/...`). This reduces ambiguity but couples behavior to executable-bit hygiene.
2. Untested claims: We validated direct invocation paths for `check_coverage_contract.sh`, `validate_defer_register.py`, and `check_bead_closure_evidence.py`; we did not add a dedicated script-mode regression test suite.
3. Nondeterminism: No runtime nondeterminism introduced. Invocation style changed only how scripts are launched, not script logic.
4. Security: No new secret/PII handling changes. Direct invocation keeps interpreter choice explicit in shebang and avoids shell-wrapper drift.
5. Performance: No material runtime impact; invocation normalization is a devex/process hardening change.

## bd-cbip · TRACK-A: process integrity hardening (bead closure evidence + script execution consistency) · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4, I5
- Constitution touched: none

1. Coupling: Governance confidence now depends on a coordinated set of audit script + CI guard + exemption ledger + invocation policy. This is intended process coupling for auditability.
2. Untested claims: We rely on end-to-end script runs and CI wiring checks rather than isolated script unit tests.
3. Nondeterminism: None added in runtime pipeline; process artifacts are deterministically generated from repo state.
4. Security: No new secrets surface. Main risk remains process misuse (stale exemptions), mitigated by explicit ledger and guard output.
5. Performance: Negligible runtime impact; small CI overhead from additional governance checks.

## bd-32uc · D4: CI perf regression policy phase-2 (fail gate after baseline lock) · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4 (deterministic evidence and enforceability)
- Constitution touched: none

1. Coupling: Full-confidence CI now couples to `docs/testing/perf-baseline-lock-v1.json` and `scripts/testing/check_perf_regression_fail.sh`. Baseline maintenance is now an operational responsibility.
2. Untested claims: Fail-gate behavior was validated locally with pass/fail/override scenarios, but CI-host variance behavior is still monitored post-merge.
3. Nondeterminism: Perf data remains environment-sensitive by nature; deterministic enforcement is achieved through locked baseline schema + explicit thresholds + explicit override reason requirement.
4. Security: No secrets introduced. Override path can be misused if left on; mitigated by required `PANOPTICON_PERF_GATE_OVERRIDE_REASON` and documented incident-only policy.
5. Performance: CI may fail more often on noisy runners once phase-2 is active; rollback path is documented and reversible by switching back to warn gate.

## bd-lh63 · TRACK-D: replay SLO and perf regression governance · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: TRACK-D now couples benchmarking, artifact schema, and CI policy into one release-quality gate path. This is intentional governance coupling for performance accountability.
2. Untested claims: We validated operational behavior in local runs and CI wiring, but long-term flake sensitivity across heterogeneous CI hardware remains a monitoring concern.
3. Nondeterminism: Performance metrics remain host-sensitive; deterministic guard behavior is enforced through pinned fixture, schema, and explicit threshold policy.
4. Security: No new secret handling introduced. Override pathway requires explicit reason to reduce silent bypass risk.
5. Performance: Adds CI runtime cost from benchmark generation and gate checks; accepted tradeoff for enterprise-grade regression visibility.

## bd-25c5 · TRACK-B: cross-provider canonical replay adapters + conformance · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I1, I4
- Constitution touched: none

1. Coupling: Adapter compatibility now spans OpenAI/Anthropic/Cohere fixture contracts and conformance drift checks. This increases schema-coupling intentionally for deterministic replay confidence.
2. Untested claims: Real-world API schema drift outside current fixture corpus remains possible; corpus updates should continue as provider contracts evolve.
3. Nondeterminism: Drift gate enforces deterministic replay/hash behavior across adapters for current fixtures; no new runtime nondeterminism introduced.
4. Security: Additional adapter fixtures increase potential secret-containing sample risk; mitigated by synthetic fixture policy and share-safe scanning downstream.
5. Performance: Conformance checks add CI runtime but are bounded and justified by cross-provider correctness guarantees.

## bd-3ta1 · C4: competitor bakeoff demo harness + proof narrative assets · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I1, I3, I4
- Constitution touched: none

1. Coupling: Bakeoff harness composes multiple demo scripts and CLI commands; output contract now depends on these surfaces staying stable.
2. Untested claims: `--full` bakeoff was executed successfully on 2026-02-18 (`.tmp/competitor-bakeoff/run-20260218T223620Z`); remaining gap is ongoing coverage against broader fixture diversity and external environment variance.
3. Nondeterminism: Report schema and required checks are deterministic for fixed inputs; runtime duration is informational only.
4. Security: Harness reuses share-safe refusal flow and synthetic fixtures; no new secret exposure path introduced.
5. Performance: Adds one more fast-demo smoke path and optional full bakeoff runtime overhead; acceptable for demo/GTM proof value.

## bd-3efx · TRACK-C: deterministic incident delta + evidence pack · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I1, I3, I4
- Constitution touched: none

1. Coupling: Compare, incident-pack, and bakeoff surfaces now share contract expectations; contract drift in one surface can break proof narratives unless checked in CI/smoke.
2. Untested claims: Incident-pack semantics are exercised on deterministic fixtures; broader real-world corpus variability still depends on fixture expansion and user bug reports.
3. Nondeterminism: Outputs are anchored to canonical ordering and deterministic hash paths; no wall-clock values are used in truth comparisons.
4. Security: Evidence-pack flow remains fail-closed through refusal semantics and scanner checks before export artifacts are consumed in demos.
5. Performance: New evidence workflows add CLI work per run but are bounded and optional; fast smoke coverage contains routine regression cost.

## bd-1zlb · PROGRAM: Deterministic Forensics Differentiation (enterprise execution track) · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I1, I2, I3, I4, I5
- Constitution touched: none

1. Coupling: Program-level closure consolidates multiple tracks that now depend on shared evidence and governance contracts; future changes must preserve these contracts across tracks.
2. Untested claims: Core contracts are covered in CI and smoke paths; long-tail provider format drift and large-corpus performance remain ongoing validation domains.
3. Nondeterminism: Canonical ordering, hash contracts, and deterministic replay checks remain enforced; no new nondeterministic inputs were introduced by closure work.
4. Security: Fail-closed export behavior, scanner refusal semantics, and integrity-oriented evidence contracts were preserved across all completed tracks.
5. Performance: Governance and proof checks add overhead but improve reliability and explainability; perf-gate phase-2 fail policy now contains regression risk.

## bd-22sa · AUDIT: random deep-code trace and fresh-eye bugfix pass · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I1, I4
- Constitution touched: none

1. Coupling: Cassette compare/incident-pack now rely on append-writer canonicalization semantics, intentionally coupling cassette normalization to the single writer path used elsewhere.
2. Untested claims: Full-path parity between cassette and eventlog inputs is now tested for clock-skew detection injection in compare mode; broader malformed-cassette edge corpus remains future work.
3. Nondeterminism: Temporary path generation uses process-id + monotonic counter only for local staging; committed output semantics remain deterministic and ordered by writer-assigned `commit_index`.
4. Security: No new secret surfaces introduced; change is confined to internal normalization path before existing export/refusal checks.
5. Performance: Cassette normalization now performs append-writer work (including detection checks), increasing compare/incident-pack cost slightly but restoring canonical truth semantics.

## bd-2zbs · AUDIT: incident-pack manifest path privacy hardening · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3
- Constitution touched: none

1. Coupling: Manifest consumers now receive stable input labels rather than raw filesystem paths, reducing accidental dependence on host-specific absolute paths.
2. Untested claims: Current tests cover typical file-path inputs; unusual edge cases (empty/invalid Unicode file names) remain handled by fallback label behavior but are not exhaustively fuzzed.
3. Nondeterminism: Path labels derive from filename components only and remain deterministic for identical input arguments.
4. Security: Eliminates local path disclosure in shareable incident-pack manifests, reducing operator and environment metadata leakage risk.
5. Performance: Negligible overhead; string extraction from path components during manifest serialization.

## bd-1bpl · AUDIT: cassette temp cleanup hardening + share-safe path edge tests · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Cassette normalization now explicitly scopes append-writer lifecycle so temporary canonicalization artifacts are cleaned on all successful return paths.
2. Untested claims: Cleanup on catastrophic process termination is still OS-dependent; this bead hardens normal/error return behavior in process, not crash-recovery semantics.
3. Nondeterminism: No truth-path nondeterminism added; test additions only validate deterministic label derivation fallback behavior.
4. Security: Keeps incident-pack privacy guarantees current by correcting stale risk language and verifying share-safe label behavior with unit tests.
5. Performance: Negligible; small control-flow restructuring and two light unit tests.

## bd-bru3 · DOCS: publish final verification evidence workflow · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: README and bakeoff doc now explicitly couple public verification steps to `.tmp/final-audit` artifact paths and existing validation scripts.
2. Untested claims: Documentation points to reproducible commands verified locally, but does not itself enforce fresh timestamps or path existence in CI.
3. Nondeterminism: No runtime behavior changes; docs describe deterministic proof surfaces and command flow only.
4. Security: Improves audit transparency by routing evidence into one known path; no new secret exposure introduced.
5. Performance: No runtime cost; minor maintenance overhead if script names or output paths change.

## bd-2mwh · DOCS: align README/showcase/demo docs with current command surface · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: README/showcase/demo docs now explicitly reference compare and incident-pack format flags plus final-audit command paths, coupling docs to current CLI/API surface.
2. Untested claims: Documentation commands were validated via test suite and existing scripts, but docs do not enforce real-time output-path existence beyond user execution.
3. Nondeterminism: No runtime changes; docs describe deterministic outputs and stable command contracts.
4. Security: Documentation clarifies that incident-pack manifest input labels are share-safe and does not advertise host-path-bearing outputs.
5. Performance: No runtime impact; slight documentation maintenance overhead as command surfaces evolve.

## bd-a9nk · DOCS: manual public-facing copy polish pass · 2026-02-18

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: Public-facing docs remain coupled to exact CLI headings and command contracts; copy edits must preserve test-bound phrases where contract tests assert literal headings.
2. Untested claims: No new product claims introduced; this pass focused on wording quality and retained existing verifiable command paths.
3. Nondeterminism: None introduced; documentation-only changes with no logic or artifact-shape modifications.
4. Security: No new sensitive content was added; wording remains aligned with share-safe and private-reporting guidance.
5. Performance: No runtime impact; only editorial maintenance overhead for future documentation refreshes.

## bd-3dv9 · SHOWCASE: asciinema capture lane (no VHS) · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: Demo/media docs now couple to `scripts/capture_showcase_cast.sh` and `asciinema` availability for optional recording.
2. Untested claims: Script behavior is validated indirectly by existing demo/test commands; no dedicated shell-level unit harness was added for cast script argument parsing.
3. Nondeterminism: Cast playback timing can vary by terminal/runtime; canonical truth artifacts remain deterministic and unchanged.
4. Security: No new secret surfaces introduced; script records existing demo commands and uses local output paths.
5. Performance: Full mode capture can be expensive due to multiple demo flows; fast mode is default to keep iteration responsive.

## bd-7l39 · DOCS: refresh showcase checklist to match shipped state · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: Checklist now tracks shipped-versus-outstanding showcase work explicitly, reducing planning drift between docs and implemented scripts/assets.
2. Untested claims: No runtime behavior claims added; checklist statements were aligned to existing files and commands already present in repo.
3. Nondeterminism: None introduced; documentation and tracker metadata only.
4. Security: No new secret exposure surfaces; edits are operational planning text.
5. Performance: No runtime impact; lower maintenance overhead from reduced stale checklist entries.

## bd-14uf · SHOWCASE: desktop/mobile proof layout refinement · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: README and showcase docs now explicitly separate desktop and narrow/mobile proof flows, coupling narrative structure to current demo command set.
2. Untested claims: New flow wording relies on existing script behavior; no additional script-level assertions were introduced for documentation semantics.
3. Nondeterminism: None added to truth path; documentation-only restructuring and command presentation updates.
4. Security: No new sensitive data handling paths were introduced; commands remain within existing share-safe and evidence workflows.
5. Performance: No runtime impact; improved scanability should reduce operator time-to-proof during demos.

## bd-2s2u · SHOWCASE: high-impact visual skin inspired by Frankentui aesthetics · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I2, I4
- Constitution touched: none

1. Coupling: Showcase profile now has stronger styling and adaptive forensic pane layout, coupling visual affordances to `UiProfile` and terminal width buckets.
2. Untested claims: Rendering behavior is covered by existing TUI/modality tests, but we did not add a new dedicated visual snapshot suite for all style states in this bead.
3. Nondeterminism: No truth-path nondeterminism introduced; changes are render-only and preserve reducer/projection/hash contracts.
4. Security: No new data paths introduced; styling and layout changes do not alter export or redaction surfaces.
5. Performance: Additional style/layout branching is minor; narrow-mode stacked forensic layout may improve readability at small widths with negligible runtime cost.

## bd-fcjd · SECURITY-3: redact absolute local paths from rendered UI/docs assets · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: TUI and export refusal-report rendering now share a path-labeling strategy (filename-first), which reduces coupling to host filesystem layout in public artifacts.
2. Untested claims: Path-label behavior is tested for normal file paths and root-like fallback cases; exotic Unicode normalization edge cases are still covered by fallback behavior but not fuzzed.
3. Nondeterminism: Label generation is deterministic for a given input path and does not affect `commit_index`, reducer state, or projection hash inputs.
4. Security: Eliminates absolute local path leakage from generated README demo artifacts and refusal reports, reducing host metadata exposure risk when publishing.
5. Performance: Negligible string/path processing overhead; no meaningful runtime impact.

## bd-1xgd · PERF-1: replace benchmark placeholder with executable Criterion lane · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: The benchmark lane now couples directly to `run_tour` and the large-stress fixture, ensuring drift is visible when tour execution semantics change.
2. Untested claims: This lane validates execution viability and core outputs, but it is not yet a full statistical regression gate with p95/p99 threshold assertions in CI.
3. Nondeterminism: Benchmark smoke reads deterministic fixture data and exercises the same deterministic tour pipeline; no truth ordering/hash behavior changed.
4. Security: No new secret/PII surfaces; benchmark output stays local and fixture-scoped.
5. Performance: Small additional compile/test surface for bench target; runtime overhead only when explicitly invoking the benchmark lane.

## bd-erqm · AUDIT: repo-wide stub/placeholder/mock gap audit · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Audit formalizes a maintenance coupling: future placeholder-like regressions should be routed through explicit beads instead of lingering in runtime paths.
2. Untested claims: Audit confirms current crate runtime paths, but third-party tooling/docs outside crate runtime are still allowed to contain planning language and historical placeholders.
3. Nondeterminism: No runtime logic changed by the audit itself; changes spawned by the audit preserved deterministic contracts.
4. Security: Audit surfaced and removed a real publish-safety leak (absolute paths in generated artifacts), improving operational privacy posture.
5. Performance: No direct runtime cost from audit process; replacement of placeholder benchmark lane adds only minimal maintenance overhead.

## bd-eyhh · ARCH-PLAN: refresh code-file reorganization plan with dependency-safe migration · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I1, I3, I4
- Constitution touched: none

1. Coupling: The plan introduces a documented coupling between future module refactors and explicit import-path migration checklists; this is intentional to reduce hidden breakage risk.
2. Untested claims: This bead is plan-only; no structural refactor was executed yet, so migration claims still require phased implementation validation.
3. Nondeterminism: No runtime code changed, and the plan explicitly preserves canonical ordering and hash-surface invariants during proposed refactors.
4. Security: No new exposure surfaces were introduced; the plan prioritizes isolation of security-sensitive export/refusal logic in smaller modules.
5. Performance: No runtime impact from this bead; minor documentation maintenance overhead as implementation phases progress.

## bd-lld3 · SECURITY-2: artifact provenance manifest for media bundle · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: Demo quickcheck now depends on `panopticon-tour --bin media_provenance` for provenance generation/verification, creating an explicit and auditable link between demo outputs and manifest integrity checks.
2. Untested claims: Unit tests cover argument parsing, deterministic ordering, and tamper detection for the provenance manifest; full end-to-end cast bundle verification is still script-driven rather than a dedicated integration test.
3. Nondeterminism: Manifest serialization is deterministic by schema and path ordering; `generated_at` is intentionally wall-clock metadata and does not feed truth-path ordering or hash contracts.
4. Security: Provenance manifest adds tamper-evident BLAKE3 checks for launch media artifacts and improves auditability of source-command lineage per asset.
5. Performance: Additional manifest generation and verification adds minor I/O and hashing overhead only during demo/release workflows, not during core runtime paths.

## bd-2asq · BUGFIX: provenance base-dir validation allows absolute in-scope asset paths · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I4
- Constitution touched: none

1. Coupling: Provenance create-mode validation now explicitly couples asset path admission to `strip_prefix(base_dir)` rather than absolute/relative syntax, which matches real script usage.
2. Untested claims: Added and exercised tests confirm traversal rejection and absolute path rejection in manifest verify mode; create-mode acceptance for in-scope absolute paths is validated by existing deterministic tests.
3. Nondeterminism: No nondeterministic behavior introduced; normalization and sorting remain deterministic.
4. Security: Maintains path safety by requiring manifest paths to be strict relative paths with no traversal components while removing an over-strict false-positive rejection.
5. Performance: Negligible impact; same path checks with corrected predicate ordering.

## bd-rl89 · SECURITY-1: media/output secret hygiene gate · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Demo quickcheck now includes an explicit dependency on the media hygiene scanner; launch/demo output workflows fail closed when secret-like tokens are detected.
2. Untested claims: We added a contract script validating pass/fail/allowlist behavior, but the scanner remains heuristic and cannot prove absence of all secret classes.
3. Nondeterminism: Scanner output ordering is deterministic by file traversal order and per-line scanning; no truth-path hashing or event ordering behavior changed.
4. Security: Introduces pre-publish secret hygiene controls with allowlist-based false-positive handling and an explicit emergency override path.
5. Performance: Adds modest extra I/O for demo/release checks only; no impact on runtime ingest/reducer/projection paths.

## bd-3w21 · MEDIA-3: launch bundle packaging + replay notes · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Launch-media packaging now depends on `scripts/demo_quickcheck.sh`, provenance verification, and media hygiene scan as a single contract chain, which intentionally couples release-demo assets to trust checks.
2. Untested claims: Contract tests validate required files and verification flow, but optional cast capture remains environment-dependent when `asciinema` is unavailable.
3. Nondeterminism: Bundle index file ordering is deterministic by sorted path traversal; git short SHA and command transcript content reflect the exact current checkout and run outputs by design.
4. Security: Bundle generation now enforces provenance and hygiene checks before success, and avoids leaking host absolute paths in `bundle-index.json` by storing a normalized run directory label.
5. Performance: Packaging adds extra hashing and command replay overhead only for launch/demo workflows; core ingest/reducer/projection runtime paths are unaffected.

## bd-3961 · MEDIA-1: trust demo cut (45-60s) · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: The trust-demo lane now couples launch messaging to deterministic tour/hash behavior and share-safe export refusal semantics, ensuring claims map directly to executable proof.
2. Untested claims: Contract coverage verifies core outputs and refusal evidence, but wall-clock runtime targeting (45-60s) is environment-dependent and not a strict timing gate.
3. Nondeterminism: Hash comparison and summary generation are deterministic for the same fixture and build; no randomness or unstable ordering was introduced in script logic.
4. Security: The lane explicitly checks refusal semantics and blocked-item evidence, reducing the risk of publishing trust claims without secret-scan enforcement proof.
5. Performance: Added script/test overhead applies only to demo/release workflows and does not affect core ingest, reducer, projection, or export runtime paths.

## bd-1t6y · MEDIA-2: visual showcase cut (45-90s) · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I2, I4
- Constitution touched: none

1. Coupling: Visual launch proof now couples to deterministic README-asset generation and explicit desktop/narrow marker checks via the visual-cut contract scripts.
2. Untested claims: We verify required assets and readability markers, but do not enforce strict wall-clock duration; runtime remains environment-dependent.
3. Nondeterminism: Visual summary hashes use deterministic file-content digesting over canonical generated assets; no truth-path ordering or hash-surface logic changed.
4. Security: The lane avoids leaking runtime host paths and validates only canonical asset surfaces; it does not alter share-safe export semantics.
5. Performance: Additional demo-contract checks run only in showcase workflows and do not affect ingest, reducer, projection, or export runtime paths.

## bd-2mt7 · SHOWCASE: launch media bundle execution · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Launch bundle execution now explicitly composes trust-cut and visual-cut lanes, creating a single reproducible command chain from proof commands to publish assets.
2. Untested claims: Contract validation ensures presence and structure of key outputs, but optional cast capture remains best-effort when `asciinema` is unavailable.
3. Nondeterminism: Bundle index and command-asset map generation use deterministic ordering for included files; timestamped output directory names remain operational metadata only.
4. Security: Bundle flow enforces provenance and hygiene checks before success and keeps path surfaces normalized to avoid host path leakage in published artifacts.
5. Performance: Extra launch checks increase demo packaging runtime only; no impact on canonical truth-path ingest/reducer/projection behavior.

## bd-eea2 · ADAPTER-1: human CLI demo track · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Human CLI demo guidance now depends on current command surface for `view`, `tour`, `export`, and `incident-pack`; CLI contract changes must keep this track aligned.
2. Untested claims: This bead is docs-only; snippet execution validity is anchored in existing command contracts and test suites, not a dedicated new snippet-runner.
3. Nondeterminism: No runtime behavior changed and no new nondeterminism surfaces were introduced.
4. Security: Track emphasizes share-safe export usage and keeps sample paths local/test-scoped; no new secret exposure surface added.
5. Performance: No runtime performance impact; only operator documentation surface changed.

## bd-20ez · ADAPTER-2: robot JSON demo track · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Robot demo guidance is now explicitly coupled to CLI envelope and exit-code contract (`panopticon-cli-robot-v1.1`); contract changes must update this track.
2. Untested claims: This bead is docs-only and relies on existing robot-mode contract tests for behavioral guarantees rather than adding new execution harnesses.
3. Nondeterminism: No runtime changes and no nondeterministic behavior introduced.
4. Security: Track keeps refusal-path and error-path expectations explicit, reducing automation ambiguity around unsafe export and invalid input handling.
5. Performance: No runtime impact; only automation documentation surface changed.

## bd-30vm · ADAPTER-3: refusal + safety demo track · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Showcase/demo docs now include a dedicated refusal-and-safety track and couple governance walkthroughs to explicit CLI envelope semantics and refusal report structure.
2. Untested claims: This bead is docs-only; command behavior is covered by existing robot-mode and export refusal contract tests, but no new doc-snippet executor was added.
3. Nondeterminism: No runtime changes and no new ordering/hash surfaces were introduced.
4. Security: The new track strengthens operator guidance for fail-closed export and runtime failure visibility; it does not introduce new data paths or secret surfaces.
5. Performance: No runtime performance impact; only documentation navigation surface changed.

## bd-2hwx · SHOWCASE: adapter-facing demo tracks · 2026-02-19

Context:
- Bead owner: Codex (gpt-5)
- Invariants referenced: I3, I4
- Constitution touched: none

1. Coupling: Showcase navigation now depends on three adapter track docs (human CLI, robot JSON, refusal/safety), which creates an explicit documentation contract for demo coverage.
2. Untested claims: This bead consolidates docs-level pathways; command behavior is validated by existing CLI/export/test harnesses rather than a new snippet runner.
3. Nondeterminism: No runtime changes and no new deterministic surfaces were introduced.
4. Security: Security posture communication improved by making refusal and runtime safety behaviors a first-class demo track, reducing operator ambiguity in public demos.
5. Performance: No runtime impact; small maintenance overhead from keeping three track docs aligned with CLI contracts.
