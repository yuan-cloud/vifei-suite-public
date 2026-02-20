# Rust Serde Determinism Guide

Byte-stable serialization is a hard requirement for Vifei (invariant I4).
This guide covers the patterns that enforce it.

**Crate versions:** serde 1.x, serde_json 1.x

---

## Rule: same data must always serialize to the same bytes

If `serialize(x) != serialize(deserialize(serialize(x)))`, you have a bug.
Round-trip byte stability is tested for every type that touches disk or
a hash input.

---

## Field ordering: struct declaration order is canonical

serde's `#[derive(Serialize)]` serializes fields in declaration order.
This is the stable guarantee we rely on. **Do not reorder struct fields
without updating round-trip tests.**

From `crates/vifei-core/src/event.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedEvent {
    pub commit_index: u64,    // ← first in JSON
    pub run_id: String,
    pub event_id: String,
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub source_seq: Option<u64>,
    pub timestamp_ns: u64,
    pub tier: Tier,
    pub payload: EventPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub payload_ref: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub synthesized: bool,
}
```

The resulting JSON field order is documented in the module doc comment:
```
commit_index, run_id, event_id, source_id, [source_seq], timestamp_ns,
tier, payload, [payload_ref], [synthesized]
```

---

## Containers: BTreeMap only, never HashMap

| Container | Deterministic? | Use in Vifei |
|-----------|---------------|-------------------|
| `BTreeMap` | Yes (sorted keys) | All map-like fields in serialized/hashed types |
| `HashMap` | **No** (random iteration) | Runtime-only state (e.g., clock skew tracker) |
| `BTreeSet` | Yes | If needed |
| `HashSet` | **No** | Never in serialized paths |
| `Vec` | Yes (ordered) | Sequences |

From `crates/vifei-core/src/event.rs`:

```rust
// GOOD: BTreeMap guarantees sorted keys in JSON
Generic {
    event_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    data: BTreeMap<String, String>,
}
```

From `crates/vifei-core/src/reducer.rs`, the entire `State` struct
uses `BTreeMap` for all map fields. No `HashMap` appears in any
serialized or hashed type.

---

## serde_json::Value: never on disk

`serde_json::Value::Object` uses insertion-order preservation, which is
fragile and nondeterministic across parse/serialize cycles.

**Rules:**
- Never use `Value` in EventLog JSONL lines
- Never use `Value` in any hashed truth surface (`state_hash`, `viewmodel.hash`)
- If dynamic JSON is unavoidable: store raw bytes as a blob, address via `payload_ref`

---

## Optional fields: skip when None, default on read

```rust
#[serde(skip_serializing_if = "Option::is_none")]
#[serde(default)]
pub source_seq: Option<u64>,
```

This keeps JSONL compact — `None` fields are omitted entirely. On
deserialization, missing fields default to `None`.

For `bool` fields that default to `false`:

```rust
#[serde(skip_serializing_if = "is_false")]
#[serde(default)]
pub synthesized: bool,

fn is_false(v: &bool) -> bool { !v }
```

---

## Internally tagged enums

`EventPayload` uses `#[serde(tag = "type")]`:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventPayload {
    RunStart { agent: String, args: Option<String> },
    RunEnd { exit_code: Option<i32>, reason: Option<String> },
    // ...
}
```

This produces: `{"type":"RunStart","agent":"agent-cli","args":null}`.
The `type` field appears first, followed by variant fields in declaration
order.

---

## Float handling: Ryu and quantization

serde_json uses the **Ryu** algorithm for `f64 → string`, which produces
the canonical shortest representation. This is deterministic for the same
f64 bit pattern.

**But:** same mathematical value can have different bit patterns (`-0.0`
vs `0.0`, NaN variants). For hashed surfaces, **quantize floats to
integers** before storing.

From `crates/vifei-core/src/reducer.rs`:

```rust
// PolicyDecision has queue_pressure: f64
// Reducer quantizes to u64 millionths for deterministic State
let clamped = queue_pressure.clamp(0.0, 1.0);
let qp_micro = (clamped * 1_000_000.0).round() as u64;
```

**Rule:** Floats are allowed in event payloads (serialized via Ryu). Floats
are forbidden in `State` and `ViewModel` (hashed surfaces). Quantize first.

---

## Key serde attributes cheat sheet

| Attribute | Purpose | When to use |
|-----------|---------|-------------|
| `#[serde(skip_serializing_if = "Option::is_none")]` | Omit None fields | All `Option<T>` in JSONL types |
| `#[serde(default)]` | Default on missing field | Pair with `skip_serializing_if` |
| `#[serde(tag = "type")]` | Internally tagged enum | `EventPayload` |
| `#[serde(rename_all = "snake_case")]` | Consistent casing | Consider for new types |
| `#[serde(deny_unknown_fields)]` | Reject unexpected input | Strict deserialization |

---

## Verification: round-trip byte stability test

Every serialized type must have this test:

```rust
fn assert_roundtrip<T: Serialize + for<'de> Deserialize<'de>>(value: &T) {
    let json1 = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&json1).unwrap();
    let json2 = serde_json::to_string(&back).unwrap();
    assert_eq!(json1, json2, "round-trip byte stability failed");
}
```

See `crates/vifei-core/src/event.rs` tests for the full set covering
all 8 Tier A variants plus Generic, payload_ref, synthesized, and edge
cases (unicode, u64::MAX, empty strings).
