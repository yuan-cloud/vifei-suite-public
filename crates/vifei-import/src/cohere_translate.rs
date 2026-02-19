//! Cohere Translate JSONL importer (v1).
//!
//! Focused on translation/compliance workflows while preserving core
//! deterministic replay guarantees.

use std::collections::BTreeMap;
use std::io::BufRead;

use serde::Deserialize;
use vifei_core::event::{EventPayload, ImportEvent, Tier};

use crate::contract::{
    contract_error_payload, normalize_event_id, normalize_run_id, reject_source_commit_index,
    validate_schema_version, COHERE_TRANSLATE_SCHEMA_VERSION,
};

/// Source identifier for events produced by this importer.
pub const SOURCE_ID: &str = "cohere-translate";

#[derive(Debug, Deserialize, Clone)]
struct TranslateRecord {
    #[serde(rename = "type")]
    event_type: Option<String>,
    schema_version: Option<String>,
    commit_index: Option<u64>,
    run_id: Option<String>,
    request_id: Option<String>,
    event_id: Option<String>,
    timestamp_ns: Option<u64>,
    created_at_ms: Option<u64>,
    model: Option<String>,
    source_lang: Option<String>,
    target_lang: Option<String>,
    source_text: Option<String>,
    translated_text: Option<String>,
    policy: Option<String>,
    policy_reason: Option<String>,
    status: Option<String>,
    error: Option<serde_json::Value>,
    queue_pressure: Option<f64>,
}

/// Parse Cohere Translate JSONL stream into [`ImportEvent`] values.
pub fn parse_cohere_translate<R: BufRead>(reader: R) -> Vec<ImportEvent> {
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

        let record: TranslateRecord = match serde_json::from_str(trimmed) {
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

fn map_record(record: &TranslateRecord, seq: u64, line_num: usize) -> ImportEvent {
    let fallback_run_id = record
        .request_id
        .as_deref()
        .unwrap_or("unknown-translate-run");
    let (run_id, _) = normalize_run_id(
        record.run_id.as_deref().or(record.request_id.as_deref()),
        fallback_run_id,
    );

    let fallback_event_id = format!("cohere:{seq}");
    let (event_id, _) = normalize_event_id(record.event_id.as_deref(), &fallback_event_id);

    let timestamp_ns = record
        .timestamp_ns
        .or_else(|| record.created_at_ms.map(|ms| ms.saturating_mul(1_000_000)))
        .unwrap_or(0);

    if let Err(message) = validate_schema_version(
        record.schema_version.as_deref(),
        COHERE_TRANSLATE_SCHEMA_VERSION,
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
    record: &TranslateRecord,
    line_num: usize,
) -> (EventPayload, Tier) {
    match event_type {
        "translation.request" => {
            let mut parts = Vec::new();
            if let Some(model) = record.model.as_deref() {
                parts.push(format!("model={model}"));
            }
            if let Some(source_lang) = record.source_lang.as_deref() {
                parts.push(format!("source_lang={source_lang}"));
            }
            if let Some(target_lang) = record.target_lang.as_deref() {
                parts.push(format!("target_lang={target_lang}"));
            }
            if let Some(text) = record.source_text.as_deref() {
                parts.push(format!("source_len={}", text.len()));
            }
            let args = if parts.is_empty() {
                None
            } else {
                Some(parts.join(","))
            };
            (
                EventPayload::RunStart {
                    agent: "cohere-translate".to_string(),
                    args,
                },
                Tier::A,
            )
        }
        "translation.result" => (
            EventPayload::ToolResult {
                tool: "translate".to_string(),
                result: record.translated_text.clone(),
                status: Some("success".to_string()),
            },
            Tier::A,
        ),
        "translation.policy" => {
            let trigger = record
                .policy_reason
                .clone()
                .or_else(|| record.policy.clone())
                .unwrap_or_else(|| "translation_policy".to_string());
            (
                EventPayload::PolicyDecision {
                    from_level: "L0".to_string(),
                    to_level: "L0".to_string(),
                    trigger,
                    queue_pressure: record.queue_pressure.unwrap_or(0.0),
                },
                Tier::A,
            )
        }
        "translation.error" => {
            let message = record
                .error
                .as_ref()
                .and_then(json_value_to_string)
                .unwrap_or_default();
            (
                EventPayload::Error {
                    kind: "provider".to_string(),
                    message,
                    severity: Some("error".to_string()),
                },
                Tier::A,
            )
        }
        "translation.completed" => (
            EventPayload::RunEnd {
                exit_code: Some(0),
                reason: record.status.clone(),
            },
            Tier::A,
        ),
        _ => {
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
    }
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
        "unknown-translate-run".to_string(),
        format!("cohere:{seq}"),
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
        let input = r#"{"type":"translation.request","request_id":"r1","event_id":"e1","created_at_ms":1000}
{"type":"translation.completed","request_id":"r1","event_id":"e2","created_at_ms":1001}
"#;
        let events = parse_cohere_translate(Cursor::new(input));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "e1");
        assert_eq!(events[1].event_id, "e2");
        assert_eq!(events[0].source_seq, Some(0));
        assert_eq!(events[1].source_seq, Some(1));
    }

    #[test]
    fn commit_index_is_rejected() {
        let input = r#"{"type":"translation.request","request_id":"r1","commit_index":9}"#;
        let events = parse_cohere_translate(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].payload, EventPayload::Error { .. }));
        if let EventPayload::Error { kind, message, .. } = &events[0].payload {
            assert_eq!(kind, "contract");
            assert!(message.contains("commit_index"));
        }
    }

    #[test]
    fn maps_policy_decision() {
        let input = r#"{"type":"translation.policy","request_id":"r1","policy":"pii_mask","queue_pressure":0.25}"#;
        let events = parse_cohere_translate(Cursor::new(input));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].payload,
            EventPayload::PolicyDecision { .. }
        ));
    }
}
