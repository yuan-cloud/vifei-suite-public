//! Agent Cassette JSONL importer -- the first and only v0.1 ingestion path.
//!
//! # Overview
//!
//! Reads Agent Cassette JSONL session recordings and maps them to
//! Vifei [`ImportEvent`] values. The importer:
//!
//! - Preserves source order exactly as received (D6). **Never** re-sorts,
//!   deduplicates, or "fixes history" based on timestamps.
//! - Marks synthesized fields with `synthesized: true` (D2).
//! - Maps recognized record types to Tier A event payloads.
//! - Falls back to `Generic` for unrecognized record types.
//! - Continues parsing on malformed lines, emitting `Error` events.
//!
//! # Agent Cassette JSONL format
//!
//! Each line is a JSON object with at minimum a `type` field. Common fields:
//!
//! | Field | Required | Description |
//! |-------|----------|-------------|
//! | `type` | yes | Record type: `session_start`, `session_end`, `tool_use`, `tool_result`, `error` |
//! | `session_id` | yes | Unique session identifier (maps to `run_id`) |
//! | `timestamp` | yes | ISO 8601 timestamp (maps to `timestamp_ns`) |
//! | `id` | no | Record identifier (maps to `event_id`) |
//!
//! # Mapping summary
//!
//! | Cassette `type` | Vifei payload | Tier |
//! |-----------------|--------------------|------|
//! | `session_start` | `RunStart` | A |
//! | `session_end` | `RunEnd` | A |
//! | `tool_use` | `ToolCall` | A |
//! | `tool_result` | `ToolResult` | A |
//! | `error` | `Error` | A |
//! | (unknown) | `Generic` | B |
//!
//! # Synthesized fields
//!
//! The `synthesized` flag is set on an event when any field is inferred:
//! - `event_id`: synthesized as `"cassette:{seq}"` when no `id` field
//! - `source_seq`: always synthesized (Agent Cassette has no sequence field)
//!
//! Since `source_seq` is always synthesized, every event from this importer
//! has `synthesized: true`. This is honest: the sequence number is our
//! invention, not present in the source data.

use std::collections::BTreeMap;
use std::io::BufRead;

use serde::Deserialize;
use vifei_core::event::{EventPayload, ImportEvent, Tier};

use crate::contract::{
    contract_error_payload, normalize_event_id, normalize_run_id, reject_source_commit_index,
    validate_schema_version, AGENT_CASSETTE_SCHEMA_VERSION,
};

/// Source identifier for events produced by this importer.
pub const SOURCE_ID: &str = "agent-cassette";

#[derive(Debug, Deserialize)]
struct CassetteRecord {
    #[serde(rename = "type")]
    record_type: Option<String>,
    schema_version: Option<String>,
    session_id: Option<String>,
    id: Option<String>,
    commit_index: Option<u64>,
    timestamp: Option<String>,
    agent: Option<String>,
    model: Option<String>,
    tool: Option<String>,
    args: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    status: Option<String>,
    exit_code: Option<i32>,
    reason: Option<String>,
    kind: Option<String>,
    message: Option<String>,
    severity: Option<String>,
}

/// Parse an Agent Cassette JSONL stream into an iterator of [`ImportEvent`].
///
/// Reads line-by-line. Malformed lines produce `Error` events rather than
/// aborting the parse. Source order is preserved exactly.
pub fn parse_cassette<R: BufRead>(reader: R) -> Vec<ImportEvent> {
    let mut events = Vec::new();
    let mut seq: u64 = 0;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                events.push(make_error_event(
                    seq,
                    &format!("IO error reading line {}: {e}", line_num + 1),
                ));
                seq += 1;
                continue;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record: CassetteRecord = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                events.push(make_error_event(
                    seq,
                    &format!("Malformed JSON at line {}: {e}", line_num + 1),
                ));
                seq += 1;
                continue;
            }
        };

        let mapped = map_record(&record, seq, line_num + 1);
        seq += 1;
        events.push(mapped);
    }

    events
}

/// Map a single Cassette JSON record to an [`ImportEvent`].
fn map_record(record: &CassetteRecord, seq: u64, line_num: usize) -> ImportEvent {
    let record_type = record.record_type.as_deref().unwrap_or("unknown");
    let (session_id, _run_synthesized) =
        normalize_run_id(record.session_id.as_deref(), "unknown-session");
    let fallback_event_id = format!("cassette:{seq}");
    let (event_id, _event_id_synthesized) =
        normalize_event_id(record.id.as_deref(), &fallback_event_id);

    let timestamp_ns = parse_timestamp_ns(record.timestamp.as_deref());

    if let Err(message) = validate_schema_version(
        record.schema_version.as_deref(),
        AGENT_CASSETTE_SCHEMA_VERSION,
    ) {
        let (payload, tier) = contract_error_payload(message);
        return ImportEvent {
            run_id: session_id,
            event_id,
            source_id: SOURCE_ID.to_string(),
            source_seq: Some(seq),
            timestamp_ns,
            tier,
            payload,
            payload_ref: None,
            synthesized: true,
        };
    }

    if let Err(message) = reject_source_commit_index(record.commit_index) {
        let (payload, tier) = contract_error_payload(message);
        return ImportEvent {
            run_id: session_id,
            event_id,
            source_id: SOURCE_ID.to_string(),
            source_seq: Some(seq),
            timestamp_ns,
            tier,
            payload,
            payload_ref: None,
            synthesized: true,
        };
    }

    let (payload, tier) = map_payload(record_type, record, seq, line_num);

    ImportEvent {
        run_id: session_id,
        event_id,
        source_id: SOURCE_ID.to_string(),
        // source_seq is always synthesized: Agent Cassette has no sequence
        // field. We assign monotonically based on parse order.
        source_seq: Some(seq),
        timestamp_ns,
        tier,
        payload,
        payload_ref: None,
        // Always true: source_seq is synthesized for every event.
        synthesized: true,
    }
}

/// Map a Cassette record type to an [`EventPayload`] and [`Tier`].
fn map_payload(
    record_type: &str,
    record: &CassetteRecord,
    seq: u64,
    line_num: usize,
) -> (EventPayload, Tier) {
    match record_type {
        "session_start" => {
            let agent = record
                .agent
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            let model = record.model.as_deref();
            let args = model.map(|m| format!("model={m}"));
            (EventPayload::RunStart { agent, args }, Tier::A)
        }

        "session_end" => {
            let exit_code = record.exit_code;
            let reason = record.reason.clone();
            (EventPayload::RunEnd { exit_code, reason }, Tier::A)
        }

        "tool_use" => {
            let tool = record.tool.clone().unwrap_or_else(|| "unknown".to_string());
            let args = record.args.as_ref().and_then(json_value_to_string);
            (EventPayload::ToolCall { tool, args }, Tier::A)
        }

        "tool_result" => {
            let tool = record.tool.clone().unwrap_or_else(|| "unknown".to_string());
            let result = record.result.as_ref().and_then(json_value_to_string);
            let status = record.status.clone();
            (
                EventPayload::ToolResult {
                    tool,
                    result,
                    status,
                },
                Tier::A,
            )
        }

        "error" => {
            let kind = record.kind.clone().unwrap_or_else(|| "unknown".to_string());
            let message = record.message.clone().unwrap_or_default();
            let severity = record.severity.clone();
            (
                EventPayload::Error {
                    kind,
                    message,
                    severity,
                },
                Tier::A,
            )
        }

        _ => {
            // Unknown record type: map to Generic with Tier B.
            let mut data = BTreeMap::new();
            data.insert("original_type".to_string(), record_type.to_string());
            data.insert("line_number".to_string(), line_num.to_string());
            data.insert("source_seq".to_string(), seq.to_string());
            (
                EventPayload::Generic {
                    event_type: record_type.to_string(),
                    data,
                },
                Tier::B,
            )
        }
    }
}

/// Convert JSON value to event payload text while preserving source fidelity.
///
/// - JSON string => raw string contents (no extra JSON quotes)
/// - JSON null => None
/// - Other JSON values => canonical JSON text
fn json_value_to_string(value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    match value.as_str() {
        Some(s) => Some(s.to_string()),
        None => Some(value.to_string()),
    }
}

/// Parse ISO 8601 timestamp to nanoseconds since Unix epoch.
///
/// Handles formats like `"2026-02-16T10:00:00.000Z"` and
/// `"2026-02-16T10:00:00Z"`. Falls back to 0 if unparseable.
fn parse_timestamp_ns(ts_str: Option<&str>) -> u64 {
    let ts_str = match ts_str {
        Some(s) => s,
        None => return 0,
    };

    // Minimal ISO 8601 parser for the subset we expect.
    // Format: YYYY-MM-DDThh:mm:ss[.fff]Z
    parse_iso8601_ns(ts_str).unwrap_or(0)
}

/// Parse a subset of ISO 8601 to nanoseconds.
fn parse_iso8601_ns(s: &str) -> Option<u64> {
    // Strip trailing Z.
    let s = s.strip_suffix('Z').or_else(|| s.strip_suffix('z'))?;

    let (date_part, time_part) = s.split_once('T')?;

    // Parse date: YYYY-MM-DD
    let mut date_parts = date_part.split('-');
    let year: u64 = date_parts.next()?.parse().ok()?;
    let month: u64 = date_parts.next()?.parse().ok()?;
    let day: u64 = date_parts.next()?.parse().ok()?;

    // Parse time: hh:mm:ss[.fractional]
    let (time_whole, frac_str) = if let Some((w, f)) = time_part.split_once('.') {
        (w, Some(f))
    } else {
        (time_part, None)
    };

    let mut time_parts = time_whole.split(':');
    let hour: u64 = time_parts.next()?.parse().ok()?;
    let minute: u64 = time_parts.next()?.parse().ok()?;
    let second: u64 = time_parts.next()?.parse().ok()?;

    // Fractional seconds → nanoseconds.
    let frac_ns: u64 = if let Some(f) = frac_str {
        parse_fractional_ns(f).unwrap_or(0)
    } else {
        0
    };

    // Days from epoch (simplified: no leap second handling).
    let days = days_from_epoch(year, month, day)?;
    let secs = days * 86400 + hour * 3600 + minute * 60 + second;
    Some(secs * 1_000_000_000 + frac_ns)
}

/// Days from Unix epoch (1970-01-01) to the given date.
/// Simplified calculation, adequate for v0.1.
fn days_from_epoch(year: u64, month: u64, day: u64) -> Option<u64> {
    if year < 1970 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }

    let days_in_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_month[m as usize] as u64;
        if m == 2 && is_leap(year) {
            days += 1;
        }
    }
    days += day - 1;

    Some(days)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Parse fractional seconds to nanoseconds with no heap allocation.
///
/// Accepts 1..N decimal digits. Uses the first 9 digits and ignores extra
/// precision, matching the previous truncate-to-9 behavior.
fn parse_fractional_ns(s: &str) -> Option<u64> {
    if s.is_empty() {
        return Some(0);
    }

    let mut value: u64 = 0;
    let mut digits_seen: usize = 0;

    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        if digits_seen < 9 {
            value = value * 10 + u64::from(b - b'0');
        }
        digits_seen += 1;
    }

    if digits_seen < 9 {
        value *= 10_u64.pow((9 - digits_seen) as u32);
    }
    Some(value)
}

/// Create an Error ImportEvent for parse failures.
fn make_error_event(seq: u64, message: &str) -> ImportEvent {
    ImportEvent {
        run_id: "unknown-session".to_string(),
        event_id: format!("cassette:{seq}"),
        source_id: SOURCE_ID.to_string(),
        source_seq: Some(seq),
        timestamp_ns: 0,
        tier: Tier::A,
        payload: EventPayload::Error {
            kind: "parse".to_string(),
            message: message.to_string(),
            severity: Some("warning".to_string()),
        },
        payload_ref: None,
        synthesized: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // -------------------------------------------------------------------
    // M3.1: Parser tests
    // -------------------------------------------------------------------

    #[test]
    fn parse_empty_input() {
        let events = parse_cassette(Cursor::new(""));
        assert!(events.is_empty());
    }

    #[test]
    fn parse_blank_lines_skipped() {
        let input = "\n\n\n";
        let events = parse_cassette(Cursor::new(input));
        assert!(events.is_empty());
    }

    #[test]
    fn parse_malformed_json_produces_error_event() {
        let input = "not json at all\n";
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error { kind, message, .. } = &events[0].payload {
            assert_eq!(kind, "parse");
            assert!(message.contains("Malformed JSON at line 1"));
        }
    }

    #[test]
    fn source_commit_index_is_rejected_by_contract() {
        let input = r#"{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"Read","commit_index":7}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error { kind, message, .. } = &events[0].payload {
            assert_eq!(kind, "contract");
            assert!(message.contains("commit_index"));
        }
    }

    #[test]
    fn schema_mismatch_is_rejected_by_contract() {
        let input = r#"{"type":"tool_use","schema_version":"agent-cassette-v999","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"Read"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error { kind, message, .. } = &events[0].payload {
            assert_eq!(kind, "contract");
            assert!(message.contains("schema_version mismatch"));
        }
    }

    #[test]
    fn parse_preserves_source_order() {
        let input = r#"{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"A","id":"t1"}
{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:02Z","tool":"B","id":"t2"}
{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:03Z","tool":"C","id":"t3"}
"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 3);

        // Verify order: A, B, C
        let tools: Vec<&str> = events
            .iter()
            .map(|e| match &e.payload {
                EventPayload::ToolCall { tool, .. } => tool.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(tools, vec!["A", "B", "C"]);

        // source_seq must be monotonic
        for (i, e) in events.iter().enumerate() {
            assert_eq!(e.source_seq, Some(i as u64));
        }
    }

    #[test]
    fn parse_does_not_sort_by_timestamp() {
        // Timestamps intentionally out of order.
        let input = r#"{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:03Z","tool":"C","id":"t3"}
{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"A","id":"t1"}
{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:02Z","tool":"B","id":"t2"}
"#;
        let events = parse_cassette(Cursor::new(input));

        // Must preserve file order: C, A, B (NOT sorted by timestamp).
        let tools: Vec<&str> = events
            .iter()
            .map(|e| match &e.payload {
                EventPayload::ToolCall { tool, .. } => tool.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(tools, vec!["C", "A", "B"]);
    }

    // -------------------------------------------------------------------
    // M3.2: Event type mapping tests
    // -------------------------------------------------------------------

    #[test]
    fn map_session_start() {
        let input = r#"{"type":"session_start","session_id":"s1","timestamp":"2026-02-16T10:00:00Z","agent":"claude-code","model":"opus-4.5"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id, "s1");
        assert_eq!(events[0].tier, Tier::A);
        assert!(matches!(&events[0].payload, EventPayload::RunStart { .. }));
        if let EventPayload::RunStart { agent, args } = &events[0].payload {
            assert_eq!(agent, "claude-code");
            assert_eq!(args.as_deref(), Some("model=opus-4.5"));
        }
    }

    #[test]
    fn map_session_end() {
        let input = r#"{"type":"session_end","session_id":"s1","timestamp":"2026-02-16T10:00:20Z","exit_code":0,"reason":"done"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tier, Tier::A);
        assert!(matches!(&events[0].payload, EventPayload::RunEnd { .. }));
        if let EventPayload::RunEnd { exit_code, reason } = &events[0].payload {
            assert_eq!(*exit_code, Some(0));
            assert_eq!(reason.as_deref(), Some("done"));
        }
    }

    #[test]
    fn map_tool_use() {
        let input = r#"{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"Read","id":"tu_001","args":{"file_path":"/foo.rs"}}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "tu_001");
        assert_eq!(events[0].tier, Tier::A);
        assert!(matches!(&events[0].payload, EventPayload::ToolCall { .. }));
        if let EventPayload::ToolCall { tool, args } = &events[0].payload {
            assert_eq!(tool, "Read");
            assert!(args
                .as_deref()
                .is_some_and(|text| text.contains("file_path")));
        }
    }

    #[test]
    fn map_tool_use_string_args_not_double_quoted() {
        let input = r#"{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"Read","args":"cat /foo.rs"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::ToolCall { .. }));
        if let EventPayload::ToolCall { args, .. } = &events[0].payload {
            assert_eq!(args.as_deref(), Some("cat /foo.rs"));
        }
    }

    #[test]
    fn map_tool_result() {
        let input = r#"{"type":"tool_result","session_id":"s1","timestamp":"2026-02-16T10:00:02Z","tool":"Read","id":"tr_001","status":"success","result":"file contents"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tier, Tier::A);
        assert!(matches!(
            &events[0].payload,
            EventPayload::ToolResult { .. }
        ));
        if let EventPayload::ToolResult {
            tool,
            result,
            status,
        } = &events[0].payload
        {
            assert_eq!(tool, "Read");
            assert_eq!(result.as_deref(), Some("file contents"));
            assert_eq!(status.as_deref(), Some("success"));
        }
    }

    #[test]
    fn map_tool_result_object_payload_preserved() {
        let input = r#"{"type":"tool_result","session_id":"s1","timestamp":"2026-02-16T10:00:02Z","tool":"Read","status":"success","result":{"ok":true,"bytes":42}}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].payload,
            EventPayload::ToolResult { .. }
        ));
        if let EventPayload::ToolResult { result, .. } = &events[0].payload {
            assert!(result.is_some());
            let rendered = result.as_deref().unwrap_or("");
            assert!(rendered.starts_with('{'));
            assert!(rendered.contains("\"ok\":true"));
            assert!(rendered.contains("\"bytes\":42"));
        }
    }

    #[test]
    fn map_error() {
        let input = r#"{"type":"error","session_id":"s1","timestamp":"2026-02-16T10:00:05Z","id":"err_001","kind":"permission","message":"Cannot write","severity":"warning"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tier, Tier::A);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error {
            kind,
            message,
            severity,
        } = &events[0].payload
        {
            assert_eq!(kind, "permission");
            assert_eq!(message, "Cannot write");
            assert_eq!(severity.as_deref(), Some("warning"));
        }
    }

    #[test]
    fn map_unknown_type_to_generic() {
        let input = r#"{"type":"heartbeat","session_id":"s1","timestamp":"2026-02-16T10:00:00Z"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tier, Tier::B);
        assert!(matches!(&events[0].payload, EventPayload::Generic { .. }));
        if let EventPayload::Generic { event_type, data } = &events[0].payload {
            assert_eq!(event_type, "heartbeat");
            assert_eq!(
                data.get("original_type").map(String::as_str),
                Some("heartbeat")
            );
        }
    }

    // -------------------------------------------------------------------
    // M3.3: Synthesized field tests
    // -------------------------------------------------------------------

    #[test]
    fn all_events_marked_synthesized() {
        let input = r#"{"type":"session_start","session_id":"s1","timestamp":"2026-02-16T10:00:00Z","agent":"test"}
{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"Bash","id":"t1"}
"#;
        let events = parse_cassette(Cursor::new(input));
        for event in &events {
            assert!(
                event.synthesized,
                "all events should have synthesized=true (source_seq is always synthesized)"
            );
        }
    }

    #[test]
    fn event_id_synthesized_when_missing() {
        let input = r#"{"type":"session_start","session_id":"s1","timestamp":"2026-02-16T10:00:00Z","agent":"test"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events[0].event_id, "cassette:0");
    }

    #[test]
    fn event_id_from_source_when_present() {
        let input = r#"{"type":"tool_use","session_id":"s1","timestamp":"2026-02-16T10:00:01Z","tool":"Bash","id":"my-custom-id"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events[0].event_id, "my-custom-id");
    }

    #[test]
    fn source_id_is_agent_cassette() {
        let input = r#"{"type":"session_start","session_id":"s1","timestamp":"2026-02-16T10:00:00Z","agent":"test"}"#;
        let events = parse_cassette(Cursor::new(input));
        assert_eq!(events[0].source_id, SOURCE_ID);
    }

    // -------------------------------------------------------------------
    // Timestamp parsing tests
    // -------------------------------------------------------------------

    #[test]
    fn parse_timestamp_with_millis() {
        let ns = parse_timestamp_ns(Some("2026-02-16T10:00:01.500Z"));
        assert!(ns > 0, "should parse to nonzero ns");
        // 500ms = 500_000_000 ns fractional
        assert_eq!(ns % 1_000_000_000, 500_000_000);
    }

    #[test]
    fn parse_timestamp_without_millis() {
        let ns = parse_timestamp_ns(Some("2026-02-16T10:00:01Z"));
        assert!(ns > 0);
        assert_eq!(ns % 1_000_000_000, 0);
    }

    #[test]
    fn parse_timestamp_missing_returns_zero() {
        assert_eq!(parse_timestamp_ns(None), 0);
    }

    #[test]
    fn parse_timestamp_fraction_truncates_after_9_digits() {
        let ns = parse_timestamp_ns(Some("2026-02-16T10:00:01.123456789987Z"));
        assert!(ns > 0);
        assert_eq!(ns % 1_000_000_000, 123_456_789);
    }

    #[test]
    fn parse_timestamp_fraction_pads_to_9_digits() {
        let ns = parse_timestamp_ns(Some("2026-02-16T10:00:01.12Z"));
        assert!(ns > 0);
        assert_eq!(ns % 1_000_000_000, 120_000_000);
    }

    #[test]
    fn parse_timestamp_fraction_invalid_returns_zero() {
        let with_invalid_fraction = parse_timestamp_ns(Some("2026-02-16T10:00:01.12xZ"));
        let with_no_fraction = parse_timestamp_ns(Some("2026-02-16T10:00:01Z"));
        assert_eq!(with_invalid_fraction, with_no_fraction);
    }

    // -------------------------------------------------------------------
    // Fixture parsing test
    // -------------------------------------------------------------------

    #[test]
    fn parse_small_session_fixture() {
        let fixture = include_str!("../../../fixtures/small-session.jsonl");
        let events = parse_cassette(Cursor::new(fixture));

        // Should have 11 events from the fixture.
        assert_eq!(events.len(), 11);

        // First event should be RunStart.
        assert!(matches!(&events[0].payload, EventPayload::RunStart { .. }));
        if let EventPayload::RunStart { agent, .. } = &events[0].payload {
            assert_eq!(agent, "claude-code");
        }

        // Last event should be RunEnd.
        assert!(matches!(&events[10].payload, EventPayload::RunEnd { .. }));
        if let EventPayload::RunEnd { exit_code, .. } = &events[10].payload {
            assert_eq!(*exit_code, Some(0));
        }

        // All events should have source_id = "agent-cassette".
        for event in &events {
            assert_eq!(event.source_id, SOURCE_ID);
        }

        // source_seq should be monotonic 0..10.
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.source_seq, Some(i as u64));
        }

        // All events should be synthesized.
        for event in &events {
            assert!(event.synthesized);
        }

        // All run_ids should be "sess-001".
        for event in &events {
            assert_eq!(event.run_id, "sess-001");
        }
    }
}
