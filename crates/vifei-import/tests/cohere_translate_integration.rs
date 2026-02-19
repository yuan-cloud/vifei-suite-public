use std::io::{BufReader, Cursor};

use vifei_core::event::EventPayload;
use vifei_import::cohere_translate::parse_cohere_translate;

#[test]
fn import_cohere_fixture() {
    let fixture = include_str!("../../../fixtures/cohere-translate-small.jsonl");
    let events = parse_cohere_translate(BufReader::new(Cursor::new(fixture)));
    assert_eq!(events.len(), 5);

    assert!(matches!(events[0].payload, EventPayload::RunStart { .. }));
    assert!(matches!(
        events[1].payload,
        EventPayload::PolicyDecision { .. }
    ));
    assert!(matches!(events[2].payload, EventPayload::ToolResult { .. }));
    assert!(matches!(events[3].payload, EventPayload::Error { .. }));
    assert!(matches!(events[4].payload, EventPayload::RunEnd { .. }));

    for (idx, event) in events.iter().enumerate() {
        assert_eq!(event.source_seq, Some(idx as u64));
        assert_eq!(event.source_id, "cohere-translate");
    }
}

#[test]
fn schema_mismatch_yields_contract_error() {
    let input = r#"{"type":"translation.request","schema_version":"cohere-translate-v999","request_id":"r1"}"#;
    let events = parse_cohere_translate(Cursor::new(input));
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].payload, EventPayload::Error { .. }));

    let (kind, message) = match &events[0].payload {
        EventPayload::Error { kind, message, .. } => (kind, message),
        _ => return,
    };
    assert_eq!(kind, "contract");
    assert!(message.contains("schema_version mismatch"));
}
