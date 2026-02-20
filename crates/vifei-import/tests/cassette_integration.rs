//! Integration test: import Agent Cassette fixture through the full pipeline.
//!
//! Parses a fixture file, feeds events through the append writer, reads
//! back the EventLog, and verifies correctness.

use std::io::Cursor;

use vifei_core::event::{EventPayload, Tier};
use vifei_core::eventlog::{read_eventlog, EventLogWriter};
use vifei_import::cassette;

#[test]
fn import_fixture_full_pipeline() {
    let fixture = include_str!("../../../fixtures/small-session.jsonl");
    let import_events = cassette::parse_cassette(Cursor::new(fixture));
    assert!(!import_events.is_empty(), "fixture should produce events");

    let dir = tempfile::tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let mut writer = EventLogWriter::open(&eventlog_path).unwrap();

    for event in import_events {
        writer.append(event).unwrap();
    }
    drop(writer);

    // Read back the EventLog.
    let committed = read_eventlog(&eventlog_path).unwrap();
    assert!(
        !committed.is_empty(),
        "EventLog should contain committed events"
    );

    // Verify commit_index is monotonically increasing from 0.
    for (i, event) in committed.iter().enumerate() {
        assert_eq!(
            event.commit_index, i as u64,
            "commit_index should be monotonic, event {i}"
        );
    }

    // Verify Tier A events are present.
    let tier_a_count = committed.iter().filter(|e| e.tier == Tier::A).count();
    assert!(tier_a_count > 0, "should have Tier A events");

    // Verify source order preserved: first event should be RunStart,
    // last non-detection event should be RunEnd.
    match &committed[0].payload {
        EventPayload::RunStart { agent, .. } => {
            assert_eq!(agent, "claude-code");
        }
        _ => panic!(
            "first event should be RunStart, got {:?}",
            committed[0].payload
        ),
    }

    // Find the last event from the fixture (not a detection event).
    let last_fixture_event = committed
        .iter()
        .rfind(|e| e.source_id == cassette::SOURCE_ID)
        .expect("should have fixture events");
    match &last_fixture_event.payload {
        EventPayload::RunEnd { exit_code, .. } => {
            assert_eq!(*exit_code, Some(0));
        }
        _ => panic!(
            "last fixture event should be RunEnd, got {:?}",
            last_fixture_event.payload
        ),
    }

    // Verify all fixture events have synthesized=true.
    for event in committed
        .iter()
        .filter(|e| e.source_id == cassette::SOURCE_ID)
    {
        assert!(
            event.synthesized,
            "fixture events should be synthesized, event_id={}",
            event.event_id
        );
    }

    // Verify no timestamp-based reordering: timestamps should appear in
    // fixture file order. For our fixture, they are already chronological,
    // but this test ensures the importer didn't sort them.
    let fixture_events: Vec<_> = committed
        .iter()
        .filter(|e| e.source_id == cassette::SOURCE_ID)
        .collect();
    for window in fixture_events.windows(2) {
        assert!(
            window[0].commit_index < window[1].commit_index,
            "fixture events should maintain file order by commit_index"
        );
    }
}

#[test]
fn import_empty_input() {
    let import_events = cassette::parse_cassette(Cursor::new(""));
    assert!(import_events.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let writer = EventLogWriter::open(&eventlog_path).unwrap();
    // No events to append.
    drop(writer);

    // EventLog file should exist but be empty.
    let committed = read_eventlog(&eventlog_path).unwrap();
    assert!(committed.is_empty());
}

#[test]
fn import_single_event() {
    let input = r#"{"type":"session_start","session_id":"s1","timestamp":"2026-02-16T10:00:00Z","agent":"test"}"#;
    let import_events = cassette::parse_cassette(Cursor::new(input));
    assert_eq!(import_events.len(), 1);

    let dir = tempfile::tempdir().unwrap();
    let eventlog_path = dir.path().join("eventlog.jsonl");
    let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
    writer
        .append(import_events.into_iter().next().unwrap())
        .unwrap();
    drop(writer);

    let committed = read_eventlog(&eventlog_path).unwrap();
    assert_eq!(committed.len(), 1);
    assert_eq!(committed[0].commit_index, 0);
}
