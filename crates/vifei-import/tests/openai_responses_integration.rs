use std::io::{BufReader, Cursor};

use vifei_core::event::EventPayload;
use vifei_import::openai_responses::parse_openai_responses;

#[test]
fn import_openai_responses_fixture() {
    let fixture = include_str!("../../../fixtures/openai-responses-small.jsonl");
    let events = parse_openai_responses(BufReader::new(Cursor::new(fixture)));
    assert_eq!(events.len(), 5);

    assert!(matches!(events[0].payload, EventPayload::RunStart { .. }));
    assert!(matches!(events[1].payload, EventPayload::ToolCall { .. }));
    assert!(matches!(events[2].payload, EventPayload::ToolResult { .. }));
    assert!(matches!(events[3].payload, EventPayload::Error { .. }));
    assert!(matches!(events[4].payload, EventPayload::RunEnd { .. }));

    for (idx, event) in events.iter().enumerate() {
        assert_eq!(event.source_seq, Some(idx as u64));
        assert_eq!(event.source_id, "openai-responses");
    }
}

#[test]
fn schema_mismatch_yields_contract_error() {
    let input = r#"{"type":"response.created","schema_version":"openai-responses-v999","response_id":"r1"}"#;
    let events = parse_openai_responses(Cursor::new(input));
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].payload, EventPayload::Error { .. }));

    let (kind, message) = match &events[0].payload {
        EventPayload::Error { kind, message, .. } => (kind, message),
        _ => return,
    };
    assert_eq!(kind, "contract");
    assert!(message.contains("schema_version mismatch"));
}
