# Best Practices Research Reference · Panopticon Suite
## Compiled: 2026-02-16T~15:00 UTC by Claude Opus 4.6 (web) for Claude Code

> **CC: Read this file FIRST before writing any guide in docs/guides/.**
> Every claim below is sourced from official docs or authoritative community refs fetched today (Feb 16, 2026).
> Do NOT hallucinate versions or APIs — use only what's listed here.

---

## 1. CRATE VERSIONS (pinned as of today)

| Crate | Latest stable | Source |
|---|---|---|
| blake3 | **1.8.3** (2026-01-08) | https://docs.rs/crate/blake3/latest |
| ratatui | **0.30.0** (latest major) | https://docs.rs/ratatui/latest/ratatui/ |
| serde | **1.x** (stable) | https://serde.rs/ |
| serde_json | **1.x** (stable) | https://docs.rs/serde_json/ |
| thiserror | **1.6.0** (2026 stable) | https://github.com/dtolnay/thiserror |
| anyhow | **1.1.0** (2026 stable) | https://github.com/dtolnay/anyhow |
| clap | **4.x** (stable) | https://docs.rs/clap/ |
| crossterm | **0.28 or 0.29** (ratatui supports both via feature flags) | ratatui release notes |

---

## 2. ANTHROPIC OFFICIAL: Claude Code Best Practices (Apr 2025, still current)

Source: https://www.anthropic.com/engineering/claude-code-best-practices

Key patterns relevant to our guides:

### CLAUDE.md files
- Place at repo root, checked into git, shared across agents
- Keep concise and human-readable
- Tune like a prompt — iterate on effectiveness
- Use `#` key to add instructions during coding sessions

### Workflow: Explore → Plan → Code → Commit
- Step 1: Read files, use subagents for complex problems
- Step 2: Plan with "think"/"think hard"/"ultrathink" (mapped to increasing thinking budget)
- Step 3: Implement with explicit verification
- Step 4: Commit + PR + update docs

### Workflow: TDD (Anthropic favorite)
- Write tests first → confirm failure → implement → iterate until pass → commit
- "Claude performs best when it has a clear target to iterate against"

### Multi-agent patterns
- Multiple git checkouts or worktrees for parallel work
- One Claude writes, another reviews
- Separate scratchpads for inter-agent communication

### Quality gates
- `cargo fmt --check`, `cargo clippy`, `cargo test` — run all three before commit
- Use checklists in markdown for complex multi-step tasks

---

## 3. ANTHROPIC: Agent Teams / C Compiler Experiment (Feb 5, 2026)

Source: https://www.anthropic.com/engineering/building-c-compiler

Directly relevant lessons for Panopticon multi-agent swarm:

### Harness design for autonomous agents
- "The task verifier must be nearly perfect, otherwise Claude will solve the wrong problem"
- Test harness prints should be concise — "do not print thousands of useless bytes"
- "Claude can't tell time and will happily spend hours running tests" → include `--fast` option with deterministic subsample
- Build CI pipeline with strict enforcement that new commits can't break existing code

### Parallelism patterns
- Agents work in independent Docker containers, claim work via lock files, push to shared git
- No central orchestrator — each agent picks the most obvious problem
- Specialization: one agent coalesces duplicate code, another improves perf, another does docs
- When many independent tests fail: trivially parallelizable (each agent picks different test)
- When one giant task (Linux kernel): use oracle comparison (GCC) + delta debugging

### Rust-specific lessons from the compiler project
- "Rust's compiler caught countless bugs that would have been far harder in dynamically typed language"
- Strong type systems + automated testing complement AI-generated code
- Code quality described as "reasonable but nowhere near expert Rust programmer"
- Human role shifts to "validation engineering" — designing harnesses, tests, success criteria

---

## 4. SERDE: Deterministic Serialization (Official + Community)

Sources: https://serde.rs/, serde data model docs, community patterns

### Field ordering guarantee
- `#[derive(Serialize)]` serializes fields in **struct declaration order** (serde default)
- This is a stable guarantee for `serde_json::to_string` — fields appear in declaration order
- **Document this as canonical** in module docs

### Containers in hashed paths
- `HashMap` → **NEVER** in serialized/hashed paths (iteration order is randomized)
- `BTreeMap` → **ALWAYS** for deterministic key ordering
- `Vec<(K, V)>` sorted → alternative if BTreeMap overhead is unwanted
- `HashSet` → same problem as HashMap. Use `BTreeSet` or sorted `Vec`

### serde_json::Value
- **NEVER on disk** in EventLog lines or hashed truth surfaces
- `Value::Object` uses `Map<String, Value>` which preserves insertion order but this is fragile
- If dynamic JSON unavoidable: store raw bytes as blob, address via `payload_ref`

### Float serialization
- serde_json uses **Ryu** algorithm for float-to-string (deterministic for same f64 bits)
- But: same mathematical value can have different bit patterns (NaN variants, -0.0 vs 0.0)
- Best practice: quantize floats before hashing, or avoid in hashed surfaces entirely
- Test with edge cases: `f64::INFINITY`, `f64::NAN`, `-0.0`, `f64::MIN_POSITIVE`

### Key serde attributes for determinism
```rust
#[serde(rename_all = "snake_case")]  // consistent casing
#[serde(deny_unknown_fields)]        // reject unexpected input
#[serde(default)]                     // explicit defaults
#[serde(skip_serializing_if = "Option::is_none")]  // canonical None handling
```

---

## 5. BLAKE3 (Official, v1.8.3)

Source: https://github.com/BLAKE3-team/BLAKE3, https://docs.rs/blake3/

### API patterns
```rust
// One-shot (preferred for small inputs)
let hash = blake3::hash(b"input");

// Incremental (for streaming / large files)
let mut hasher = blake3::Hasher::new();
hasher.update(b"chunk1");
hasher.update(b"chunk2");
let hash = hasher.finalize();

// Print as lowercase hex
println!("{}", hash);  // Display impl outputs lowercase hex

// To hex string
let hex: String = hash.to_hex().to_string();
```

### Content addressing pattern for blob store
```rust
fn store_blob(data: &[u8], blob_dir: &Path) -> std::io::Result<String> {
    let hash = blake3::hash(data);
    let hex = hash.to_hex().to_string();  // lowercase hex
    let blob_path = blob_dir.join(&hex);
    if !blob_path.exists() {
        std::fs::write(&blob_path, data)?;
        // fsync for durability (Panopticon requirement)
    }
    Ok(hex)
}
```

### Key facts
- Output: 256 bits (32 bytes) by default, but extensible (XOF)
- Deterministic: same input always produces same output (no salt/seed by default)
- Keyed mode: `blake3::keyed_hash(&key, data)` — NOT needed for content addressing
- `derive_key` mode: for KDF — NOT needed for Panopticon v0.1
- **NOT a password hash** (too fast) — not relevant but worth noting
- Latest release 1.8.3 fixed serialization backwards compatibility issue

---

## 6. RATATUI (Official, v0.30.0)

Sources: https://ratatui.rs/, https://docs.rs/ratatui/latest/ratatui/, https://ratatui.rs/highlights/v030/

### v0.30.0 breaking changes (CRITICAL for Panopticon)
- **Modular workspace**: split into `ratatui`, `ratatui-core`, `ratatui-widgets`
  - Apps should still depend on main `ratatui` crate (re-exports everything)
  - Widget library authors should depend on `ratatui-core` for API stability
- **`Alignment` renamed to `HorizontalAlignment`** (type alias kept for compat)
- **MSRV: Rust 1.86.0**
- **Crossterm feature flags**: `crossterm_0_28` and `crossterm_0_29` — default is latest
- **`ratatui::run()`** new convenience method for simple apps
- **`no_std` support** added (not relevant for Panopticon)
- `Frame::size()` deprecated → use `Frame::area()`
- `WidgetRef` blanket impl reversed

### Application patterns (from official docs)
1. **Immediate mode rendering**: redraw every frame, no retained state in UI
2. **Flux architecture**: recommended for complex apps (unidirectional data flow)
   - Action → Dispatcher → Store → View → (repeat)
   - Maps well to Panopticon: EventLog → Reducer → Projection → ViewModel → TUI
3. **MVC pattern**: also documented as viable
4. **Actor pattern**: for tokio-based apps (Panopticon is not async in v0.1)

### Testing with TestBackend
- `TestBackend` allows asserting buffer contents after draw
- Widget tests: render to TestBackend, assert cell contents
- Snapshot testing: serialize buffer to string, compare against golden file

### Layout system
- `Layout::default().direction(Direction::Vertical).constraints([...]).split(area)`
- Constraints: `Constraint::Percentage`, `Min`, `Max`, `Length`, `Ratio`, `Fill`
- Nested layouts for responsive design

### Key widgets for Panopticon
- `Table` — for event lists in Forensic Lens
- `Paragraph` — for event detail inspector
- `Block` — for lens framing
- `Gauge` / `LineGauge` — for queue pressure in Truth HUD
- `Tabs` — for Incident/Forensic lens toggle
- `Scrollbar` — for timeline scrubbing

---

## 7. ERROR HANDLING (thiserror + anyhow, 2026 best practices)

Sources: dtolnay repos, community consensus as of Jan-Feb 2026

### The Rule
- **Library crates** (`panopticon-core`, `-import`, `-export`): use `thiserror`
- **Binary crate** (`panopticon-tui` main.rs): use `anyhow`
- Many projects use BOTH — thiserror for structured errors, anyhow at the top

### thiserror patterns (v1.6.0)
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventLogError {
    #[error("append stalled for {duration_ms}ms (limit: {limit_ms}ms)")]
    AppendStall { duration_ms: u64, limit_ms: u64 },
    
    #[error("blob write failed: {0}")]
    BlobWrite(#[from] std::io::Error),
    
    #[error("event line exceeds max bytes: {size} > {max}")]
    OversizedEvent { size: usize, max: usize },
    
    #[error("clock skew detected: source {source_id} moved backward by {delta_ns}ns")]
    ClockSkew { source_id: String, delta_ns: i64 },
}
```

### anyhow patterns (v1.1.0)
```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let eventlog = open_eventlog(&path)
        .with_context(|| format!("Failed to open eventlog at {}", path.display()))?;
    // ...
    Ok(())
}
```

### Anti-patterns to avoid
- Don't use `anyhow` in library crate public APIs — callers can't match on error variants
- Don't create too many thiserror variants — group related errors
- Always derive `Debug` on error types
- Always use `#[source]` or `#[from]` to preserve error chains
- Never `.unwrap()` in library code — use `expect()` only for truly impossible states with explanation
- **Panopticon-specific**: FM-* failure modes (FM-APPEND-FAIL, FM-BLOB-WRITE-FAIL) must map to specific thiserror variants, not generic anyhow errors

### ResExt (new, Feb 2026)
- A new crate offering anyhow-like ergonomics with thiserror-like stack-based performance
- Too new for Panopticon v0.1 — mention as "watch this space" only

---

## 8. CLAP (v4.x, stable)

### Subcommand pattern for Panopticon CLI
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "panopticon", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import an Agent Cassette session
    Import {
        /// Input file (Agent Cassette JSONL)
        #[arg(value_name = "FILE")]
        input: PathBuf,
    },
    /// View an EventLog in the TUI
    View {
        /// EventLog file
        #[arg(value_name = "FILE")]
        eventlog: PathBuf,
    },
    /// Export a share-safe bundle
    Export {
        /// Output bundle path
        #[arg(short, long)]
        output: PathBuf,
        /// EventLog file
        #[arg(value_name = "FILE")]
        eventlog: PathBuf,
        /// Enable share-safe mode (required)
        #[arg(long)]
        share_safe: bool,
    },
    /// Run the Tour stress harness
    Tour {
        /// Enable stress mode
        #[arg(long)]
        stress: bool,
        /// Input fixture
        #[arg(value_name = "FILE")]
        fixture: PathBuf,
    },
    /// Rebuild the SQLite index cache
    Reindex {
        /// EventLog file
        #[arg(value_name = "FILE")]
        eventlog: PathBuf,
    },
}
```

### Exit code conventions
- 0: success
- 1: general error
- 2: usage error (clap handles this)
- 3: export refused (secrets detected) — Panopticon-specific

---

## 9. SECURITY / REDACTION PATTERNS

No single authoritative source — synthesized from security scanning best practices:

### Secret detection patterns (for M8 export scanner)
- High-entropy string detection (Shannon entropy > 4.5 on base64-like strings)
- Known patterns: AWS keys (`AKIA...`), GitHub tokens (`ghp_...`, `gho_...`), JWT (`eyJ...`)
- Regex-based pattern matching with configurable rule sets
- API key formats: `sk-...`, `pk-...`, `Bearer ...`

### Deterministic masking
- Replace matched content with fixed-length placeholder: `[REDACTED:pattern_name]`
- Masking must be deterministic: same input → same output (for bundle_hash stability)
- Mask in-place in event payloads, not just in export — the refusal report references original positions

### Refusal report schema (from PLANS.md)
- Already fully specified in PLANS.md § "Artifact schema contracts"
- Guide should reference that schema, not duplicate it

---

## 10. CROSS-CUTTING: Lessons from Anthropic C Compiler for Multi-Agent Panopticon

These are the most actionable takeaways for our upcoming 3-agent swarm:

1. **Test harness quality > code quality**: "The task verifier must be nearly perfect"
2. **Print concise output**: Don't flood agent context with verbose test output
3. **Deterministic subsampling**: Use `--fast` flags for quick iteration, full suite in CI
4. **Lock-file coordination**: Our Agent Mail + beads system is more sophisticated than what Carlini used
5. **Specialization works**: Assign different concerns to different agents (our Track A/B/C)
6. **CI as gatekeeper**: New commits must not break existing tests — our quality gates already enforce this
7. **Strong types catch bugs**: Rust's type system is a force multiplier for agent-written code
8. **Context management**: `/clear` between tasks, use scratchpads for long chains
