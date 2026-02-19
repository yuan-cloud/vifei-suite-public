# CLI Design Guide

Patterns for the Vifei command-line interface using Clap v4 derive API.

**Crate version:** clap 4.x (stable)

---

## Subcommand structure

The CLI maps directly to the hero loop in `PLANS.md`:

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vifei", version, about = "Terminal cockpit for AI agent runs")]
struct Cli {
    /// Emit machine-readable JSON output
    #[arg(long, global = true, conflicts_with = "human")]
    json: bool,
    /// Force human-readable output
    #[arg(long, global = true)]
    human: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import an Agent Cassette session into an EventLog
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
        /// Enable share-safe mode (required for export)
        #[arg(long)]
        share_safe: bool,
    },
    /// Run the Tour stress harness
    Tour {
        /// Enable stress mode
        #[arg(long)]
        stress: bool,
        /// Input fixture file
        #[arg(value_name = "FILE")]
        fixture: PathBuf,
    },
    /// Rebuild the SQLite index cache from EventLog
    Reindex {
        /// EventLog file
        #[arg(value_name = "FILE")]
        eventlog: PathBuf,
    },
}
```

---

## Hero loop mapping

```text
1) vifei import cassette ./session.jsonl
2) vifei view ./eventlog.jsonl
3) [Tab] toggle Incident ↔ Forensic Lens
4) vifei export --share-safe -o ./bundle.tar.zst ./eventlog.jsonl
5) vifei tour --stress ./fixtures/large-session.jsonl
```

Each step maps to one `Commands` variant.

---

## Exit codes

| Code | Meaning | When |
|------|---------|------|
| 0 | Success | Normal completion |
| 1 | Not found | Required input path missing |
| 2 | Usage error | Invalid arguments / parse failure |
| 3 | Export refused | Secrets detected during share-safe export |
| 4 | Runtime error | IO errors, parse failures after parse stage, runtime failures |

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.is_export_refused() => {
            eprintln!("Export refused: {e}");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
```

---

## UX conventions

### Output philosophy

- **Structured output for machines:** Tour artifacts are JSON, hashes are
  plain text. Parseable by CI.
- **Robot mode for agents:** `--json` emits compact envelopes with
  `ok`, `code`, `message`, and `suggestions`.
- **TTY-aware default:** if stdout is not a TTY, auto-switch to JSON
  unless `--human` is explicitly set.
- **Human-readable stderr for operators:** Progress, warnings, and errors
  go to stderr.
- **Minimal stdout:** Only the primary output (e.g., bundle path, hash).

### Progress reporting

```
Importing session.jsonl... 1042 events
Writing EventLog... done (1042 committed, 0 clock skew)
```

Keep it one line per phase. Do not print thousands of lines — agents
cannot parse verbose output efficiently (per Anthropic agent team lessons).

### Error messages

Follow the pattern: what happened, why, how to fix.

```
Error: EventLog line exceeds 1048576 bytes at commit_index 4221
  Event payload is too large for inline storage.
  Store large payloads as blobs using payload_ref.
```

Robot-mode error envelope (JSON):

```json
{
  "schema_version": "vifei-cli-robot-v1.1",
  "ok": false,
  "code": "INVALID_ARGS",
  "message": "Invalid command syntax.",
  "suggestions": [
    "Run `vifei --help` for command syntax.",
    "Run `vifei <command> --help` for command-specific args."
  ],
  "exit_code": 2
}
```

Robot-mode success envelope (JSON):

```json
{
  "schema_version": "vifei-cli-robot-v1.1",
  "ok": true,
  "code": "OK",
  "message": "Tour completed successfully.",
  "suggestions": [],
  "exit_code": 0,
  "command": "tour",
  "data": {
    "output_dir": "tour-output",
    "event_count": 10000,
    "tier_a_drops": 0,
    "degradation_level": "L2",
    "viewmodel_hash": "..."
  }
}
```

Contract requirements:

- `schema_version`, `ok`, `code`, `message`, `suggestions`, and `exit_code` are required in every JSON envelope.
- `data` is required on success envelopes.
- `notes` is optional and used only when normalization/repair was applied.
- `command` is optional metadata for command-scoped responses.

### Intent-repair policy

Allowed:

- unambiguous command aliases (for example `viewer` -> `view`)
- underscore-to-hyphen flag variants (`--output_dir` -> `--output-dir`)

Not allowed:

- guessing between multiple possible commands
- silently changing mutating command intent

If intent is not unambiguous, fail with structured guidance and examples.

### Two-layer failure contract

Use this matrix as the authoritative behavior split between parsing and execution:

| Layer | Condition | Behavior | Expected envelope |
|---|---|---|---|
| Parser-repair boundary | Unambiguous normalization (`--output_dir` -> `--output-dir`, known alias) | Repair is allowed, but must be explicit | `OK`/error with `notes` entry describing normalization |
| Parser boundary | Ambiguous/invalid syntax | No guessing | `INVALID_ARGS` with actionable suggestions |
| Execution boundary | Runtime artifact write/serialize failure | No placeholder fallback, no silent downgrade | `RUNTIME_ERROR` with stable `exit_code` |
| Execution boundary | Share-safe export scanner findings | Hard refusal path | `EXPORT_REFUSED` with refusal-report guidance |

Reference contract tests:
- `crates/vifei-tui/tests/cli_robot_mode_contract.rs`
- `crates/vifei-tui/src/main.rs` (`parse_error_guidance_*`, normalization tests)

---

## Argument validation

Use clap's built-in validation where possible:

```rust
/// Import an Agent Cassette session
Import {
    /// Input file (must exist and be readable)
    #[arg(value_name = "FILE", value_parser = clap::value_parser!(PathBuf))]
    input: PathBuf,
}
```

For complex validation (e.g., file must exist, must be valid JSONL),
validate after parsing and return clear errors via `anyhow`.

---

## Testing CLI parsing

```rust
#[test]
fn test_import_parsing() {
    let cli = Cli::try_parse_from(["vifei", "import", "session.jsonl"]).unwrap();
    match cli.command {
        Commands::Import { input } => assert_eq!(input, PathBuf::from("session.jsonl")),
        _ => panic!("expected Import"),
    }
}

#[test]
fn test_export_requires_share_safe() {
    let cli = Cli::try_parse_from([
        "vifei", "export", "--share-safe", "-o", "out.tar.zst", "eventlog.jsonl"
    ]).unwrap();
    match cli.command {
        Commands::Export { share_safe, .. } => assert!(share_safe),
        _ => panic!("expected Export"),
    }
}
```
