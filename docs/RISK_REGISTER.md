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
