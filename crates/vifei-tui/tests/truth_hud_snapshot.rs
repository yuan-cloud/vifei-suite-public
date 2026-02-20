//! Truth HUD snapshot test — M6.6 acceptance criteria.
//!
//! Exercises the full pipeline: EventLog → reduce → project → render → buffer.
//! Asserts all 6 required Truth HUD fields are present in rendered output.
//!
//! If someone removes a field from the Truth HUD or breaks its wiring
//! in the main render function, this test fails.

use vifei_core::event::{EventPayload, ImportEvent, Tier};
use vifei_core::eventlog::EventLogWriter;
use vifei_tui::render_to_buffer;

/// Create a minimal fixture event for the EventLog.
fn fixture_event(id: &str, ts: u64) -> ImportEvent {
    ImportEvent {
        run_id: "run-fixture".into(),
        event_id: id.into(),
        source_id: "snapshot-test".into(),
        source_seq: Some(0),
        timestamp_ns: ts,
        tier: Tier::A,
        payload: EventPayload::RunStart {
            agent: "test-agent".into(),
            args: None,
        },
        payload_ref: None,
        synthesized: false,
    }
}

/// Full pipeline snapshot: all 6 required Truth HUD fields must appear.
///
/// Required fields (per BACKPRESSURE_POLICY projection invariants):
/// 1. Current degradation ladder level (L0..L5)
/// 2. Aggregation mode + bin size
/// 3. Backlog / queue pressure indicator
/// 4. Tier A drops counter
/// 5. Export safety state
/// 6. Projection invariants version
#[test]
fn truth_hud_all_fields_present() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.jsonl");

    // Write a minimal EventLog fixture
    let mut writer = EventLogWriter::open(&path).unwrap();
    writer.append(fixture_event("e1", 1_000_000_000)).unwrap();
    writer.append(fixture_event("e2", 2_000_000_000)).unwrap();
    drop(writer);

    // Render full pipeline to buffer (120x24 terminal)
    let text = render_to_buffer(&path, 120, 24).unwrap();

    // Assert all 6 required fields
    assert!(
        text.contains("Level:"),
        "Missing degradation level label in Truth HUD"
    );
    assert!(
        text.contains("L0"),
        "Missing degradation level value (default L0)"
    );
    assert!(
        text.contains("Agg:"),
        "Missing aggregation mode label in Truth HUD"
    );
    assert!(
        text.contains("1:1"),
        "Missing aggregation mode value (default 1:1)"
    );
    assert!(
        text.contains("Pressure:"),
        "Missing queue pressure label in Truth HUD"
    );
    assert!(
        text.contains("0%"),
        "Missing queue pressure value (default 0%)"
    );
    assert!(
        text.contains("Drops:"),
        "Missing Tier A drops label in Truth HUD"
    );
    assert!(
        text.contains("Export:"),
        "Missing export safety state label in Truth HUD"
    );
    assert!(
        text.contains("UNKNOWN"),
        "Missing export safety state value (default UNKNOWN)"
    );
    assert!(
        text.contains("Version:"),
        "Missing projection invariants version label in Truth HUD"
    );
    assert!(
        text.contains("projection-invariants-v0.1"),
        "Missing projection invariants version value"
    );
}

/// Truth HUD must be present in BOTH lenses.
/// The default is Incident Lens; this test also validates the HUD
/// is rendered as part of the main render pipeline (not just truth_hud.rs).
#[test]
fn truth_hud_present_in_default_lens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.jsonl");
    let mut writer = EventLogWriter::open(&path).unwrap();
    writer.append(fixture_event("e1", 1_000_000_000)).unwrap();
    drop(writer);

    let text = render_to_buffer(&path, 120, 24).unwrap();

    // Truth HUD block title must appear
    assert!(
        text.contains("Truth HUD"),
        "Truth HUD block title missing from rendered output"
    );

    // Incident Lens must also appear (default lens)
    assert!(
        text.contains("Incident Lens"),
        "Incident Lens should be the default view"
    );
}

/// Snapshot test with empty EventLog — HUD must still render with defaults.
#[test]
fn truth_hud_renders_with_empty_eventlog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.jsonl");
    let writer = EventLogWriter::open(&path).unwrap();
    drop(writer);

    let text = render_to_buffer(&path, 120, 24).unwrap();

    // Even with no events, HUD must show all required fields
    assert!(
        text.contains("Level:"),
        "HUD must render even with empty EventLog"
    );
    assert!(
        text.contains("Version:"),
        "HUD version must render even with empty EventLog"
    );
    assert!(
        text.contains("projection-invariants-v0.1"),
        "HUD version value must be present with empty EventLog"
    );
}

/// The projection invariants version string must be an exact match.
/// This catches accidental version bumps or typos.
#[test]
fn truth_hud_version_exact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.jsonl");
    let mut writer = EventLogWriter::open(&path).unwrap();
    writer.append(fixture_event("e1", 1_000_000_000)).unwrap();
    drop(writer);

    let text = render_to_buffer(&path, 120, 24).unwrap();

    assert!(
        text.contains("projection-invariants-v0.1"),
        "Exact version string 'projection-invariants-v0.1' must appear in HUD"
    );
}
