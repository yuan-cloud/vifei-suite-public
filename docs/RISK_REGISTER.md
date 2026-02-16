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
