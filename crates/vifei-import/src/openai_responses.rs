//! OpenAI Responses JSONL importer (v1).
//!
//! This parser is intentionally conservative and deterministic:
//! - preserves source order exactly,
//! - rejects source-provided `commit_index`,
//! - maps high-value event families to Tier A payloads,
//! - emits `Generic` Tier B for unknown event shapes.

use std::collections::BTreeMap;
use std::io::BufRead;

use serde::Deserialize;
use vifei_core::event::{EventPayload, ImportEvent, Tier};

use crate::contract::{
    contract_error_payload, normalize_event_id, normalize_run_id, reject_source_commit_index,
    validate_schema_version, OPENAI_RESPONSES_SCHEMA_VERSION,
};

/// Source identifier for events produced by this importer.
pub const SOURCE_ID: &str = "openai-responses";

#[derive(Debug, Deserialize, Clone)]
struct ResponsesRecord {
    #[serde(rename = "type")]
    event_type: Option<String>,
    schema_version: Option<String>,
    commit_index: Option<u64>,
    run_id: Option<String>,
    response_id: Option<String>,
    event_id: Option<String>,
    timestamp_ns: Option<u64>,
    created_at_ms: Option<u64>,
    model: Option<String>,
    status: Option<String>,
    error: Option<serde_json::Value>,
    item: Option<serde_json::Value>,
}

/// Parse OpenAI Responses JSONL stream into [`ImportEvent`] values.
pub fn parse_openai_responses<R: BufRead>(reader: R) -> Vec<ImportEvent> {
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

        let record: ResponsesRecord = match serde_json::from_str(trimmed) {
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

fn map_record(record: &ResponsesRecord, seq: u64, line_num: usize) -> ImportEvent {
    let fallback_run_id = record
        .response_id
        .as_deref()
        .unwrap_or("unknown-response-run");
    let (run_id, _) = normalize_run_id(
        record.run_id.as_deref().or(record.response_id.as_deref()),
        fallback_run_id,
    );

    let fallback_event_id = format!("openai:{seq}");
    let candidate_event_id = record
        .event_id
        .as_deref()
        .or_else(|| record.item.as_ref().and_then(item_id));
    let (event_id, _) = normalize_event_id(candidate_event_id, &fallback_event_id);

    let timestamp_ns = record
        .timestamp_ns
        .or_else(|| record.created_at_ms.map(|ms| ms.saturating_mul(1_000_000)))
        .unwrap_or(0);

    if let Err(message) = validate_schema_version(
        record.schema_version.as_deref(),
        OPENAI_RESPONSES_SCHEMA_VERSION,
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
    record: &ResponsesRecord,
    line_num: usize,
) -> (EventPayload, Tier) {
    match event_type {
        "response.created" => {
            let model = record.model.as_deref();
            let args = model.map(|m| format!("model={m}"));
            (
                EventPayload::RunStart {
                    agent: "openai-responses".to_string(),
                    args,
                },
                Tier::A,
            )
        }
        "response.completed" => (
            EventPayload::RunEnd {
                exit_code: Some(0),
                reason: record.status.clone(),
            },
            Tier::A,
        ),
        "response.error" => {
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
        _ => match map_item_payload(record.item.as_ref()) {
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

fn map_item_payload(item: Option<&serde_json::Value>) -> Option<(EventPayload, Tier)> {
    let item = item?;
    let item_type = item.get("type")?.as_str()?;
    match item_type {
        "function_call" => {
            let tool = item
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let args = item.get("arguments").and_then(json_value_to_string);
            Some((EventPayload::ToolCall { tool, args }, Tier::A))
        }
        "function_call_output" => {
            let tool = item
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let result = item.get("output").and_then(json_value_to_string);
            Some((
                EventPayload::ToolResult {
                    tool,
                    result,
                    status: Some("success".to_string()),
                },
                Tier::A,
            ))
        }
        _ => None,
    }
}

fn item_id(item: &serde_json::Value) -> Option<&str> {
    item.get("id").and_then(serde_json::Value::as_str)
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
        "unknown-response-run".to_string(),
        format!("openai:{seq}"),
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
        let input = r#"{"type":"response.created","response_id":"r1","event_id":"e1","created_at_ms":1000}
{"type":"response.completed","response_id":"r1","event_id":"e2","created_at_ms":1001}
"#;
        let events = parse_openai_responses(Cursor::new(input));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "e1");
        assert_eq!(events[1].event_id, "e2");
        assert_eq!(events[0].source_seq, Some(0));
        assert_eq!(events[1].source_seq, Some(1));
    }

    #[test]
    fn commit_index_is_rejected() {
        let input = r#"{"type":"response.created","response_id":"r1","commit_index":5}"#;
        let events = parse_openai_responses(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error { kind, message, .. } = &events[0].payload {
            assert_eq!(kind, "contract");
            assert!(message.contains("commit_index"));
        }
    }

    #[test]
    fn maps_function_call_and_output() {
        let input = r#"{"type":"response.output_item.added","response_id":"r1","item":{"id":"it1","type":"function_call","name":"search","arguments":{"q":"rust"}}}
{"type":"response.output_item.done","response_id":"r1","item":{"id":"it2","type":"function_call_output","name":"search","output":"ok"}}
"#;
        let events = parse_openai_responses(Cursor::new(input));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].payload, EventPayload::ToolCall { .. }));
        assert!(matches!(events[1].payload, EventPayload::ToolResult { .. }));
    }
}
