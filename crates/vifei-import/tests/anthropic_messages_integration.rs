use std::io::{BufReader, Cursor};

use vifei_core::event::EventPayload;
use vifei_import::anthropic_messages::parse_anthropic_messages;
use vifei_import::openai_responses::parse_openai_responses;

#[test]
fn import_anthropic_fixture() {
    let fixture = include_str!("../../../fixtures/anthropic-messages-small.jsonl");
    let events = parse_anthropic_messages(BufReader::new(Cursor::new(fixture)));
    assert_eq!(events.len(), 5);

    assert!(matches!(events[0].payload, EventPayload::RunStart { .. }));
    assert!(matches!(events[1].payload, EventPayload::ToolCall { .. }));
    assert!(matches!(events[2].payload, EventPayload::ToolResult { .. }));
    assert!(matches!(events[3].payload, EventPayload::Error { .. }));
    assert!(matches!(events[4].payload, EventPayload::RunEnd { .. }));

    for (idx, event) in events.iter().enumerate() {
        assert_eq!(event.source_seq, Some(idx as u64));
        assert_eq!(event.source_id, "anthropic-messages");
    }
}

#[test]
fn schema_mismatch_yields_contract_error() {
    let input =
        r#"{"type":"message_start","schema_version":"anthropic-messages-v999","message_id":"m1"}"#;
    let events = parse_anthropic_messages(Cursor::new(input));
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].payload, EventPayload::Error { .. }));

    let (kind, message) = match &events[0].payload {
        EventPayload::Error { kind, message, .. } => (kind, message),
        _ => return,
    };
    assert_eq!(kind, "contract");
    assert!(message.contains("schema_version mismatch"));
}

#[test]
fn overlapping_semantics_match_openai_shape() {
    let openai_input = r#"{"type":"response.created","response_id":"r1","event_id":"o1","created_at_ms":1000,"model":"gpt-5-mini"}
{"type":"response.output_item.added","response_id":"r1","event_id":"o2","item":{"id":"oi1","type":"function_call","name":"search","arguments":{"q":"deterministic replay"}}}
{"type":"response.output_item.done","response_id":"r1","event_id":"o3","item":{"id":"oi2","type":"function_call_output","name":"search","output":"ok"}}
{"type":"response.completed","response_id":"r1","event_id":"o4","created_at_ms":1001,"status":"completed"}
"#;
    let anthropic_input = r#"{"type":"message_start","message_id":"m1","event_id":"a1","created_at_ms":1000,"model":"claude-3-5-sonnet"}
{"type":"content_block_start","message_id":"m1","event_id":"a2","content_block":{"id":"ac1","type":"tool_use","name":"search","input":{"q":"deterministic replay"}}}
{"type":"content_block_stop","message_id":"m1","event_id":"a3","content_block":{"id":"ac2","type":"tool_result","name":"search","content":"ok"}}
{"type":"message_stop","message_id":"m1","event_id":"a4","created_at_ms":1001,"stop_reason":"end_turn"}
"#;

    let openai_events = parse_openai_responses(Cursor::new(openai_input));
    let anthropic_events = parse_anthropic_messages(Cursor::new(anthropic_input));
    assert_eq!(openai_events.len(), anthropic_events.len());

    let openai_shapes: Vec<&'static str> = openai_events.iter().map(payload_shape).collect();
    let anthropic_shapes: Vec<&'static str> = anthropic_events.iter().map(payload_shape).collect();
    assert_eq!(openai_shapes, anthropic_shapes);
}

fn payload_shape(event: &vifei_core::event::ImportEvent) -> &'static str {
    match &event.payload {
        EventPayload::RunStart { .. } => "run_start",
        EventPayload::ToolCall { .. } => "tool_call",
        EventPayload::ToolResult { .. } => "tool_result",
        EventPayload::RunEnd { .. } => "run_end",
        EventPayload::Error { .. } => "error",
        EventPayload::PolicyDecision { .. } => "policy_decision",
        EventPayload::RedactionApplied { .. } => "redaction",
        EventPayload::ClockSkewDetected { .. } => "clock_skew",
        EventPayload::Generic { .. } => "generic",
    }
}
