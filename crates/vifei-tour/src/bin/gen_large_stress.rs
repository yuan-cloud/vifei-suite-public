//! Deterministic large stress fixture generator for Tour (M7.2).
//!
//! Produces `fixtures/large-stress.jsonl` — a 10K+ event Agent Cassette JSONL
//! fixture with realistic characteristics for stress testing the Tour pipeline.
//!
//! # Determinism
//!
//! Uses xorshift64 with fixed seed `0xDEAD_BEEF_CAFE_1234`. Same binary →
//! same fixture, always.
//!
//! # Usage
//!
//! ```sh
//! cargo run --bin gen-large-stress
//! ```

use serde_json::json;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Xorshift64 PRNG — deterministic, no external dependencies.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Random u64 in [min, max).
    fn range(&mut self, min: u64, max: u64) -> u64 {
        min + self.next_u64() % (max - min)
    }

    /// Pick an element from a slice.
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.next_u64() as usize % items.len();
        &items[idx]
    }
}

/// Convert nanoseconds since Unix epoch to ISO 8601 string.
fn ns_to_iso8601(ns: u64) -> String {
    let total_secs = ns / 1_000_000_000;
    let frac_ms = (ns % 1_000_000_000) / 1_000_000;

    let mut remaining_days = (total_secs / 86400) as i64;
    let time_secs = total_secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Compute year from days since 1970-01-01
    let mut year = 1970i32;
    loop {
        let days_in_year: i64 = if is_leap(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Compute month and day
    let month_days = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &md in &month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, frac_ms
    )
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Generate a tool-appropriate args object.
fn gen_args(rng: &mut Rng, tool: &str) -> serde_json::Value {
    match tool {
        "Read" => json!({"file_path": format!("/project/src/mod_{}.rs", rng.range(1, 50))}),
        "Edit" => json!({
            "file_path": format!("/project/src/mod_{}.rs", rng.range(1, 50)),
            "old_string": "old_value",
            "new_string": "new_value"
        }),
        "Write" => {
            let size = rng.range(10, 200) as usize;
            let content: String = (0..size).map(|_| 'x').collect();
            json!({"file_path": format!("/project/src/gen_{}.rs", rng.range(1, 30)), "content": content})
        }
        "Bash" => {
            let cmds = [
                "cargo test",
                "cargo build",
                "ls -la",
                "git status",
                "cargo clippy",
            ];
            json!({"command": *rng.pick(&cmds)})
        }
        "Glob" => json!({"pattern": format!("**/*.{}", rng.pick(&["rs", "toml", "md"]))}),
        "Grep" => json!({"pattern": format!("fn {}", rng.pick(&["main", "new", "test", "run"]))}),
        "WebSearch" => json!({"query": "rust async patterns"}),
        _ => json!({}),
    }
}

/// Generate a tool-appropriate result string with varying payload sizes.
fn gen_result(rng: &mut Rng, tool: &str, payload_class: u64) -> String {
    match payload_class {
        // Small payload (~10-50 bytes)
        0 => match tool {
            "Edit" => "Edit applied successfully".into(),
            "Bash" => "ok".into(),
            "Glob" => "src/main.rs".into(),
            _ => "ok".into(),
        },
        // Medium payload (~100-500 bytes)
        1 => {
            let lines = rng.range(3, 10);
            (0..lines)
                .map(|i| format!("line {}: {}", i, "a".repeat(rng.range(10, 60) as usize)))
                .collect::<Vec<_>>()
                .join("\n")
        }
        // Large payload (~1000-4000 bytes, near inline threshold)
        _ => {
            let lines = rng.range(20, 80);
            (0..lines)
                .map(|i| {
                    format!(
                        "    fn method_{}(&self) -> Result<(), Error> {{ Ok(()) }}",
                        i
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn main() {
    let output_path = "fixtures/large-stress.jsonl";

    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);

    let agents = ["claude-code", "codex-cli", "cursor", "aider"];
    let models = [
        "claude-opus-4-6",
        "gpt-5-codex",
        "cursor-fast",
        "aider-v0.50",
    ];
    let tools = ["Read", "Edit", "Write", "Bash", "Glob", "Grep", "WebSearch"];
    let error_kinds = ["permission", "timeout", "runtime", "validation", "network"];
    let error_messages = [
        "Cannot write to /etc/hosts",
        "Operation timed out after 30s",
        "Unexpected null reference",
        "Schema validation failed",
        "Connection refused: port 5432",
    ];

    let file = File::create(output_path).expect("Failed to create fixture file");
    let mut writer = BufWriter::new(file);

    let mut total_events: u64 = 0;

    // Base time: 2026-01-15T00:00:00Z in nanoseconds
    // 2026-01-15 = 20468 days since epoch
    // 20468 * 86400 = 1_768_435_200 seconds
    let base_ns: u64 = 1_768_435_200 * 1_000_000_000;
    let mut current_ns = base_ns;

    // 25 sessions to ensure >= 10K events with varying sizes
    let num_sessions = 25u64;

    for run_idx in 0..num_sessions {
        let session_id = format!("stress-{:03}", run_idx);
        let agent_idx = run_idx as usize % agents.len();
        let agent = agents[agent_idx];
        let model = models[agent_idx];

        // Vary pairs per session: 150-550 (average ~350, total ~25*350*2 + overhead ≈ 17,500+)
        let pairs = rng.range(150, 550);

        // session_start
        let event = json!({
            "type": "session_start",
            "session_id": session_id,
            "timestamp": ns_to_iso8601(current_ns),
            "agent": agent,
            "model": model,
        });
        writeln!(writer, "{}", event).unwrap();
        current_ns += rng.range(500, 2000) * 1_000_000; // 0.5-2s
        total_events += 1;

        for pair_idx in 0..pairs {
            let tool = *rng.pick(&tools);
            let tu_id = format!("tu_{:03}_{:04}", run_idx, pair_idx);
            let tr_id = format!("tr_{:03}_{:04}", run_idx, pair_idx);

            // tool_use
            let args = gen_args(&mut rng, tool);
            let tu_event = json!({
                "type": "tool_use",
                "session_id": session_id,
                "timestamp": ns_to_iso8601(current_ns),
                "tool": tool,
                "id": tu_id,
                "args": args,
            });
            writeln!(writer, "{}", tu_event).unwrap();
            let tool_use_ns = current_ns;
            current_ns += rng.range(100, 5000) * 1_000_000; // 100ms-5s
            total_events += 1;

            // tool_result — inject backward timestamp for clock skew in some runs
            let result_ns = if pair_idx == pairs / 3 && run_idx % 5 == 0 {
                // Backward by 2s (well above 50ms tolerance) to trigger ClockSkewDetected
                tool_use_ns.saturating_sub(2_000_000_000)
            } else {
                current_ns
            };

            // Vary payload sizes: ~60% small, ~30% medium, ~10% large
            let payload_class = match rng.range(0, 10) {
                0..=5 => 0, // small
                6..=8 => 1, // medium
                _ => 2,     // large
            };
            let result = gen_result(&mut rng, tool, payload_class);

            let tr_event = json!({
                "type": "tool_result",
                "session_id": session_id,
                "timestamp": ns_to_iso8601(result_ns),
                "tool": tool,
                "id": tr_id,
                "tool_use_id": tu_id,
                "status": "success",
                "result": result,
            });
            writeln!(writer, "{}", tr_event).unwrap();
            current_ns += rng.range(50, 2000) * 1_000_000; // 50ms-2s
            total_events += 1;

            // Occasional error (~3% chance)
            if rng.range(0, 100) < 3 {
                let err_id = format!("err_{:03}_{:04}", run_idx, pair_idx);
                let kind = *rng.pick(&error_kinds);
                let message = *rng.pick(&error_messages);
                let severity = *rng.pick(&["warning", "error", "critical"]);

                let err_event = json!({
                    "type": "error",
                    "session_id": session_id,
                    "timestamp": ns_to_iso8601(current_ns),
                    "id": err_id,
                    "kind": kind,
                    "message": message,
                    "severity": severity,
                });
                writeln!(writer, "{}", err_event).unwrap();
                current_ns += rng.range(200, 1000) * 1_000_000; // 200ms-1s
                total_events += 1;
            }
        }

        // session_end
        let exit_code = if rng.range(0, 10) < 8 { 0 } else { 1 };
        let reason = if exit_code == 0 {
            "Task completed successfully"
        } else {
            "Terminated due to errors"
        };
        let end_event = json!({
            "type": "session_end",
            "session_id": session_id,
            "timestamp": ns_to_iso8601(current_ns),
            "exit_code": exit_code,
            "reason": reason,
        });
        writeln!(writer, "{}", end_event).unwrap();
        total_events += 1;

        // Gap between sessions: 10-60s
        current_ns += rng.range(10, 60) * 1_000_000_000;
    }

    writer.flush().unwrap();

    eprintln!("Generated {} events to {}", total_events, output_path);
    eprintln!("Sessions: {}", num_sessions);
    eprintln!(
        "Time span: {} → {}",
        ns_to_iso8601(base_ns),
        ns_to_iso8601(current_ns)
    );
}
