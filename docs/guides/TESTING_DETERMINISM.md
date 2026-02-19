# Testing Determinism Guide

Determinism is not vibes. It is bytes. This guide covers the testing
patterns that enforce invariant I4 (testable determinism).

---

## Round-trip byte stability

The foundational test: serialize, deserialize, re-serialize, assert
byte equality.

From `crates/vifei-core/src/event.rs`:

```rust
fn assert_roundtrip<T: Serialize + for<'de> Deserialize<'de>>(value: &T, label: &str) {
    let json1 = serde_json::to_string(value)
        .unwrap_or_else(|e| panic!("{label}: serialize failed: {e}"));
    let back: T = serde_json::from_str(&json1)
        .unwrap_or_else(|e| panic!("{label}: deserialize failed: {e}"));
    let json2 = serde_json::to_string(&back)
        .unwrap_or_else(|e| panic!("{label}: re-serialize failed: {e}"));
    assert_eq!(json1, json2, "{label}: round-trip byte stability failed");
}
```

**Every type that touches disk or a hash input must have this test.**

### What to cover

- All enum variants (M1 covers all 8 Tier A variants + Generic)
- Optional fields present and absent (`payload_ref`, `source_seq`)
- Boolean fields true and false (`synthesized`)
- Edge cases: empty strings, unicode, `u64::MAX`

---

## Hash stability across runs

The N-run test catches hidden nondeterminism (HashMap iteration, thread-local
state, float accumulation order).

From `crates/vifei-core/src/reducer.rs`:

```rust
#[test]
fn determinism_10_runs() {
    let events: Vec<_> = /* fixed event sequence covering all variants */;
    let mut hashes = Vec::new();
    for _ in 0..10 {
        let (state, _) = replay(&events);
        hashes.push(state_hash(&state));
    }
    for (i, hash) in hashes.iter().enumerate() {
        assert_eq!(hash, &hashes[0], "Run {i} produced different state_hash");
    }
}
```

**Why 10 runs?** HashMap nondeterminism is probabilistic. A single run
might coincidentally match. 10 runs makes false-pass probability negligible.

**If flaky:** A flaky result is a REAL determinism bug, not a test issue.

---

## Checkpoint rebuild equivalence

Full replay must equal checkpoint + remaining replay:

```rust
#[test]
fn checkpoint_rebuild_equivalence() {
    let events = /* 6000 events (crosses 5000 checkpoint boundary) */;

    // Full replay
    let (state_full, _) = replay(&events);

    // Checkpoint at 5000, then resume
    let (state_at_cp, _) = replay(&events[..5000]);
    let checkpoint = create_checkpoint(&state_at_cp);
    let loaded = load_checkpoint(&serialize_checkpoint(&checkpoint)).unwrap();
    let (state_from_cp, _) = replay_from(loaded.state, &events[5000..]);

    assert_eq!(state_full, state_from_cp);
    assert_eq!(state_hash(&state_full), state_hash(&state_from_cp));
}
```

Test at multiple boundaries: exact interval, interval + 1, 2x interval.

---

## Float determinism with Ryu

serde_json uses Ryu for canonical shortest `f64` representation. Test
with specific values that are known to have clean representations:

```rust
#[test]
fn policy_decision_float_determinism() {
    let values = [0.0, 0.5, 0.8, 0.85, 1.0, 0.123456789];
    for qp in values {
        let event = make_import_event(EventPayload::PolicyDecision {
            from_level: "L0".into(), to_level: "L1".into(),
            trigger: "test".into(), queue_pressure: qp,
        });
        assert_roundtrip(&event, &format!("PolicyDecision qp={qp}"));
    }
}
```

For hashed surfaces (State, ViewModel): **quantize floats to integers**
before storing. See `RUST_SERDE_DETERMINISM.md` § "Float handling".

---

## Field order verification

Assert that serialized JSON has fields in the documented canonical order:

```rust
#[test]
fn committed_event_field_order() {
    let json = serde_json::to_string(&event).unwrap();
    let ci_pos = json.find("\"commit_index\"").unwrap();
    let ri_pos = json.find("\"run_id\"").unwrap();
    assert!(ci_pos < ri_pos, "commit_index before run_id");
    // ... continue for all fields
}
```

---

## BTreeMap key ordering

Verify sorted keys appear in sorted order in JSON output:

```rust
#[test]
fn state_uses_btreemap_only() {
    let mut state = State::new();
    state.event_counts_by_type.insert("Zebra".into(), 1);
    state.event_counts_by_type.insert("Alpha".into(), 2);
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.find("\"Alpha\"").unwrap() < json.find("\"Zebra\"").unwrap());
}
```

---

## JSONL format verification

Events must serialize to single-line compact JSON:

```rust
#[test]
fn committed_event_jsonl_no_pretty_print() {
    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.contains('\n'), "JSONL must not contain newlines");
    assert!(!json.contains("  "), "JSONL must not be pretty-printed");
}
```

---

## Snapshot testing (for TUI, M6)

Use `TestBackend` to capture rendered output and compare against golden
files. Changes to golden files must be reviewed — they represent
intentional UI changes.

---

## Lessons from Anthropic agent teams

From the [C compiler experiment](https://www.anthropic.com/engineering/building-c-compiler):

1. **"The task verifier must be nearly perfect."** Round-trip and hash
   stability tests ARE the verifier for determinism. Invest in their
   coverage.
2. **Deterministic subsampling.** Use `--fast` flags for quick iteration
   (small fixture), full suite in CI (large fixture from
   `docs/CAPACITY_ENVELOPE.md` targets).
3. **Strong types catch bugs.** `BTreeMap` instead of `HashMap`, two-type
   pattern for `commit_index` — the type system prevents entire classes
   of nondeterminism.

---

## Test naming conventions

```
test_roundtrip_{variant}        # byte stability
test_determinism_{n}_runs       # hash stability
test_checkpoint_rebuild_{case}  # checkpoint equivalence
test_{type}_field_order         # canonical ordering
```
