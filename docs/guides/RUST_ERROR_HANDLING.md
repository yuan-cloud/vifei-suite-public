# Rust Error Handling Guide

Error handling patterns for Vifei using thiserror and anyhow.

**Crate versions:** thiserror 1.6.0, anyhow 1.1.0

---

## The rule

| Crate type | Error library | Why |
|-----------|--------------|-----|
| Library (`vifei-core`, `-import`, `-export`) | `thiserror` | Callers can match on specific variants |
| Binary (`vifei-tui` main.rs) | `anyhow` | Ergonomic, context-rich error chains |

Many projects use both. thiserror defines structured errors in libraries;
anyhow wraps them at the top level for human-readable reporting.

---

## thiserror patterns (library crates)

### Vifei-specific error enum

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventLogError {
    /// FM-APPEND-FAIL: append writer cannot durably append Tier A
    #[error("append stalled for {duration_ms}ms (limit: {limit_ms}ms)")]
    AppendStall { duration_ms: u64, limit_ms: u64 },

    /// FM-BLOB-WRITE-FAIL: blob cannot be durably written
    #[error("blob write failed: {0}")]
    BlobWrite(#[from] std::io::Error),

    /// Event line exceeds max bytes budget
    #[error("event line exceeds max bytes: {size} > {max}")]
    OversizedEvent { size: usize, max: usize },

    /// Serialization failure
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### FM-* failure mode mapping

Each failure mode from `docs/BACKPRESSURE_POLICY.md` § "Failure modes v0.1"
maps to a specific thiserror variant. Do not use generic `anyhow` errors
for failure modes — callers need to match on them to enter the correct
safe failure posture.

| Failure mode | Error variant | Safe failure action |
|-------------|--------------|---------------------|
| FM-APPEND-FAIL | `AppendStall` | Enter L5 immediately |
| FM-BLOB-WRITE-FAIL | `BlobWrite` | Emit Error event if possible, then L5 |
| FM-PROJECTION-OVERBUDGET | `ProjectionOverbudget` | Follow ladder L0→L4 |
| FM-EXPORT-UNSAFE | `ExportRefused` | Emit refusal report, exit code 3 |

### Import errors

```rust
#[derive(Error, Debug)]
pub enum ImportError {
    #[error("malformed JSONL at line {line}: {message}")]
    MalformedLine { line: usize, message: String },

    #[error("unknown record type: {record_type}")]
    UnknownRecordType { record_type: String },

    #[error("IO error reading source: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## anyhow patterns (binary crate)

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let eventlog = open_eventlog(&path)
        .with_context(|| format!("Failed to open eventlog at {}", path.display()))?;

    let events = read_eventlog(&eventlog_path)
        .context("Failed to read events from EventLog")?;

    Ok(())
}
```

### Context is key

Always add context when propagating errors. Bare `?` loses the "what
were we doing when this failed" information.

```rust
// BAD: bare ? loses context
let file = File::open(&path)?;

// GOOD: context explains what we were trying to do
let file = File::open(&path)
    .with_context(|| format!("opening EventLog at {}", path.display()))?;
```

---

## Never `.unwrap()` in library code

| Method | Use in library code | Use in tests |
|--------|-------------------|-------------|
| `.unwrap()` | Never | Always fine |
| `.expect("reason")` | Only for truly impossible states | Always fine |
| `?` with thiserror | Preferred | Fine |
| `.ok()?` | For optional fallbacks | Fine |

```rust
// BAD: panics in production
let data = serde_json::to_vec(&state).unwrap();

// GOOD: returns error to caller
let data = serde_json::to_vec(&state)?;

// ACCEPTABLE: truly impossible state, documented
let data = serde_json::to_vec(&state)
    .expect("State contains only safe serde types");
```

---

## Error chain preservation

Always use `#[source]` or `#[from]` to preserve the error chain:

```rust
#[derive(Error, Debug)]
pub enum ExportError {
    // #[from] automatically implements From<io::Error> and sets #[source]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Manual #[source] when you need custom wrapping
    #[error("checkpoint load failed for {path}")]
    CheckpointLoad {
        path: String,
        #[source]
        cause: serde_json::Error,
    },
}
```

---

## Error grouping

Don't create too many variants. Group related errors:

```rust
// BAD: too granular
enum Error {
    FileNotFound(PathBuf),
    FilePermission(PathBuf),
    FileCorrupt(PathBuf),
    FileTooLarge(PathBuf),
}

// GOOD: grouped with context
enum Error {
    #[error("EventLog error at {path}: {kind}")]
    EventLog { path: PathBuf, kind: String, #[source] cause: std::io::Error },
}
```

---

## Testing error paths

```rust
#[test]
fn oversized_event_rejected() {
    let result = writer.append(huge_event);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("exceeds max bytes"));
}
```

Use `assert!(result.is_err())` and check the error message or variant.
Do not just test happy paths.

---

## Watch: ResExt (Feb 2026) — anyhow ergonomics + thiserror performance. Too new for v0.1.
