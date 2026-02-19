//! Anthropic messages/tool-use JSONL importer (v1).
//!
//! Deterministic guarantees:
//! - preserves source order exactly,
//! - rejects source-provided `commit_index`,
//! - maps overlapping run/tool semantics to canonical Tier A payloads,
//! - emits `Generic` Tier B for unknown shapes.

use std::collections::BTreeMap;
use std::io::BufRead;

use serde::Deserialize;
use vifei_core::event::{EventPayload, ImportEvent, Tier};

use crate::contract::{
    contract_error_payload, normalize_event_id, normalize_run_id, reject_source_commit_index,
    validate_schema_version, ANTHROPIC_MESSAGES_SCHEMA_VERSION,
};

/// Source identifier for events produced by this importer.
pub const SOURCE_ID: &str = "anthropic-messages";

#[derive(Debug, Deserialize, Clone)]
struct AnthropicRecord {
    #[serde(rename = "type")]
    event_type: Option<String>,
    schema_version: Option<String>,
    commit_index: Option<u64>,
    run_id: Option<String>,
    session_id: Option<String>,
    message_id: Option<String>,
    event_id: Option<String>,
    timestamp_ns: Option<u64>,
    created_at_ms: Option<u64>,
    model: Option<String>,
    status: Option<String>,
    stop_reason: Option<String>,
    error: Option<serde_json::Value>,
    item: Option<serde_json::Value>,
    content_block: Option<serde_json::Value>,
    delta: Option<serde_json::Value>,
    content: Option<serde_json::Value>,
}

/// Parse Anthropic JSONL stream into [`ImportEvent`] values.
pub fn parse_anthropic_messages<R: BufRead>(reader: R) -> Vec<ImportEvent> {
    let mut events = Vec::new();
    let mut seq: u64 = 0;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(v) => v,
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

        let record: AnthropicRecord = match serde_json::from_str(trimmed) {
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

        events.push(map_record(&record, seq, line_num + 1));
        seq += 1;
    }

    events
}

fn map_record(record: &AnthropicRecord, seq: u64, line_num: usize) -> ImportEvent {
    let fallback_run_id = record
        .message_id
        .as_deref()
        .unwrap_or("unknown-anthropic-run");
    let (run_id, _) = normalize_run_id(
        record
            .run_id
            .as_deref()
            .or(record.session_id.as_deref())
            .or(record.message_id.as_deref()),
        fallback_run_id,
    );

    let fallback_event_id = format!("anthropic:{seq}");
    let candidate_event_id = record
        .event_id
        .as_deref()
        .or_else(|| candidate_item_id(record))
        .or(record.message_id.as_deref());
    let (event_id, _) = normalize_event_id(candidate_event_id, &fallback_event_id);

    let timestamp_ns = record
        .timestamp_ns
        .or_else(|| record.created_at_ms.map(|ms| ms.saturating_mul(1_000_000)))
        .unwrap_or(0);

    if let Err(message) = validate_schema_version(
        record.schema_version.as_deref(),
        ANTHROPIC_MESSAGES_SCHEMA_VERSION,
    ) {
        let (payload, tier) = contract_error_payload(message);
        return as_event(run_id, event_id, seq, timestamp_ns, tier, payload, true);
    }

    if let Err(message) = reject_source_commit_index(record.commit_index) {
        let (payload, tier) = contract_error_payload(message);
        return as_event(run_id, event_id, seq, timestamp_ns, tier, payload, true);
    }

    let event_type = record.event_type.as_deref().unwrap_or("unknown");
    let (payload, tier) = map_payload(event_type, record, line_num);
    as_event(run_id, event_id, seq, timestamp_ns, tier, payload, true)
}

fn map_payload(
    event_type: &str,
    record: &AnthropicRecord,
    line_num: usize,
) -> (EventPayload, Tier) {
    match event_type {
        "message_start" | "message.created" => {
            let args = record.model.as_ref().map(|m| format!("model={m}"));
            (
                EventPayload::RunStart {
                    agent: "anthropic-messages".to_string(),
                    args,
                },
                Tier::A,
            )
        }
        "message_stop" | "message.completed" => {
            let reason = record.stop_reason.clone().or_else(|| record.status.clone());
            (
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason,
                },
                Tier::A,
            )
        }
        "error" | "message.error" => {
            let rendered = record
                .error
                .as_ref()
                .and_then(json_value_to_string)
                .unwrap_or_default();
            (
                EventPayload::Error {
                    kind: "provider".to_string(),
                    message: rendered,
                    severity: Some("error".to_string()),
                },
                Tier::A,
            )
        }
        _ => match map_tool_payload(record) {
            Some(mapped) => mapped,
            None => {
                let mut data = BTreeMap::new();
                data.insert("event_type".to_string(), event_type.to_string());
                data.insert("line_number".to_string(), line_num.to_string());
                (
                    EventPayload::Generic {
                        event_type: event_type.to_string(),
                        data,
                    },
                    Tier::B,
                )
            }
        },
    }
}

fn map_tool_payload(record: &AnthropicRecord) -> Option<(EventPayload, Tier)> {
    for value in candidate_tool_values(record) {
        let item_type = value.get("type")?.as_str()?;
        match item_type {
            "tool_use" => {
                let tool = value
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let args = value
                    .get("input")
                    .or_else(|| value.get("arguments"))
                    .and_then(json_value_to_string);
                return Some((EventPayload::ToolCall { tool, args }, Tier::A));
            }
            "tool_result" => {
                let tool = value
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| value.get("tool_name").and_then(serde_json::Value::as_str))
                    .or_else(|| value.get("tool_use_id").and_then(serde_json::Value::as_str))
                    .unwrap_or("unknown")
                    .to_string();
                let result = value
                    .get("content")
                    .or_else(|| value.get("output"))
                    .and_then(json_value_to_string);
                let status = match value.get("is_error").and_then(serde_json::Value::as_bool) {
                    Some(true) => Some("error".to_string()),
                    _ => Some("success".to_string()),
                };
                return Some((
                    EventPayload::ToolResult {
                        tool,
                        result,
                        status,
                    },
                    Tier::A,
                ));
            }
            _ => {}
        }
    }
    None
}

fn candidate_tool_values(record: &AnthropicRecord) -> Vec<&serde_json::Value> {
    let mut out = Vec::new();
    if let Some(v) = record.item.as_ref() {
        out.push(v);
    }
    if let Some(v) = record.content_block.as_ref() {
        out.push(v);
    }
    if let Some(v) = record.delta.as_ref() {
        out.push(v);
    }
    if let Some(serde_json::Value::Array(items)) = record.content.as_ref() {
        for item in items {
            out.push(item);
        }
    }
    out
}

fn candidate_item_id(record: &AnthropicRecord) -> Option<&str> {
    let values = candidate_tool_values(record);
    for value in values {
        if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
            return Some(id);
        }
    }
    None
}

fn json_value_to_string(value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    match value.as_str() {
        Some(s) => Some(s.to_string()),
        None => Some(value.to_string()),
    }
}

fn as_event(
    run_id: String,
    event_id: String,
    seq: u64,
    timestamp_ns: u64,
    tier: Tier,
    payload: EventPayload,
    synthesized: bool,
) -> ImportEvent {
    ImportEvent {
        run_id,
        event_id,
        source_id: SOURCE_ID.to_string(),
        source_seq: Some(seq),
        timestamp_ns,
        tier,
        payload,
        payload_ref: None,
        synthesized,
    }
}

fn make_error_event(seq: u64, message: &str) -> ImportEvent {
    as_event(
        "unknown-anthropic-run".to_string(),
        format!("anthropic:{seq}"),
        seq,
        0,
        Tier::A,
        EventPayload::Error {
            kind: "parse".to_string(),
            message: message.to_string(),
            severity: Some("warning".to_string()),
        },
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_preserves_source_order() {
        let input = r#"{"type":"message_start","message_id":"m1","event_id":"e1","created_at_ms":1000}
{"type":"message_stop","message_id":"m1","event_id":"e2","created_at_ms":1001}
"#;
        let events = parse_anthropic_messages(Cursor::new(input));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "e1");
        assert_eq!(events[1].event_id, "e2");
        assert_eq!(events[0].source_seq, Some(0));
        assert_eq!(events[1].source_seq, Some(1));
    }

    #[test]
    fn commit_index_is_rejected() {
        let input = r#"{"type":"message_start","message_id":"m1","commit_index":3}"#;
        let events = parse_anthropic_messages(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error { kind, message, .. } = &events[0].payload {
            assert_eq!(kind, "contract");
            assert!(message.contains("commit_index"));
        }
    }

    #[test]
    fn maps_tool_use_and_result() {
        let input = r#"{"type":"content_block_start","message_id":"m1","content_block":{"id":"cb1","type":"tool_use","name":"search","input":{"q":"rust"}}}
{"type":"content_block_stop","message_id":"m1","content_block":{"id":"cb2","type":"tool_result","name":"search","content":"ok"}}
"#;
        let events = parse_anthropic_messages(Cursor::new(input));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].payload, EventPayload::ToolCall { .. }));
        assert!(matches!(events[1].payload, EventPayload::ToolResult { .. }));
    }
}
