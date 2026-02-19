//! Integration tests for the large stress fixture (M7.2).
//!
//! Validates that `fixtures/large-stress.jsonl` meets all CAPACITY_ENVELOPE
//! requirements and can be processed through the Tour pipeline.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use vifei_tour::{TourConfig, TourMetrics};

/// Path to the large stress fixture.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/large-stress.jsonl")
}

/// Parse fixture JSONL into a vec of serde_json::Value.
fn parsed_fixture() -> &'static Vec<serde_json::Value> {
    static PARSED_FIXTURE: OnceLock<Vec<serde_json::Value>> = OnceLock::new();
    PARSED_FIXTURE.get_or_init(|| {
        let content = fs::read_to_string(fixture_path()).expect("fixture must exist");
        content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).expect("each line must be valid JSON"))
            .collect()
    })
}

// --- Fixture characteristic tests ---

#[test]
fn fixture_has_at_least_10k_events() {
    let events = parsed_fixture();
    assert!(
        events.len() >= 10_000,
        "CAPACITY_ENVELOPE requires >= 10,000 events, got {}",
        events.len()
    );
}

#[test]
fn fixture_has_representative_event_mix() {
    let events = parsed_fixture();
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for event in events {
        let t = event["type"].as_str().unwrap().to_string();
        *type_counts.entry(t).or_default() += 1;
    }

    // All required event types must be present
    for required in &[
        "session_start",
        "session_end",
        "tool_use",
        "tool_result",
        "error",
    ] {
        assert!(
            type_counts.contains_key(*required),
            "Missing required event type: {}",
            required
        );
    }

    // tool_use and tool_result should be the dominant types
    let tu = type_counts.get("tool_use").copied().unwrap_or(0);
    let tr = type_counts.get("tool_result").copied().unwrap_or(0);
    assert!(tu > 5000, "Expected >5000 tool_use events, got {}", tu);
    assert!(tr > 5000, "Expected >5000 tool_result events, got {}", tr);
    assert_eq!(tu, tr, "tool_use and tool_result counts must match");
}

#[test]
fn fixture_has_multiple_runs() {
    let events = parsed_fixture();
    let sessions: HashSet<String> = events
        .iter()
        .filter_map(|e| e["session_id"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        sessions.len() >= 5,
        "Expected multiple runs (>= 5 sessions), got {}",
        sessions.len()
    );
}

#[test]
fn fixture_has_multiple_agents() {
    let events = parsed_fixture();
    let agents: HashSet<String> = events
        .iter()
        .filter(|e| e["type"] == "session_start")
        .filter_map(|e| e["agent"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        agents.len() >= 2,
        "Expected multiple agents (>= 2), got {:?}",
        agents
    );
}

#[test]
fn fixture_has_backward_timestamps() {
    let events = parsed_fixture();
    let mut backward_count = 0;
    let mut prev_ts = String::new();
    let mut prev_session = String::new();

    for event in events {
        let ts = event["timestamp"].as_str().unwrap_or("").to_string();
        let session = event["session_id"].as_str().unwrap_or("").to_string();

        if session == prev_session && ts < prev_ts {
            backward_count += 1;
        }
        prev_ts = ts;
        prev_session = session;
    }

    assert!(
        backward_count >= 1,
        "Expected at least 1 backward timestamp for clock skew, found {}",
        backward_count
    );
}

#[test]
fn fixture_has_varying_payload_sizes() {
    let events = parsed_fixture();
    let mut sizes: Vec<usize> = events
        .iter()
        .filter(|e| e["type"] == "tool_result")
        .filter_map(|e| e["result"].as_str().map(|s| s.len()))
        .collect();
    sizes.sort();

    assert!(!sizes.is_empty(), "No tool_result payloads found");

    let small = sizes.iter().filter(|&&s| s < 50).count();
    let large = sizes.iter().filter(|&&s| s > 1000).count();

    assert!(
        small > 100,
        "Expected >100 small payloads (<50 bytes), got {}",
        small
    );
    assert!(
        large > 50,
        "Expected >50 large payloads (>1KB), got {}",
        large
    );
}

#[test]
fn fixture_is_valid_jsonl() {
    let content = fs::read_to_string(fixture_path()).expect("fixture must exist");
    let mut line_num = 0;
    for line in content.lines() {
        line_num += 1;
        if line.is_empty() {
            continue;
        }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "Invalid JSON on line {}: {}",
            line_num,
            parsed.unwrap_err()
        );
        let val = parsed.unwrap();
        assert!(
            val["type"].is_string(),
            "Missing 'type' field on line {}",
            line_num
        );
        assert!(
            val["session_id"].is_string(),
            "Missing 'session_id' field on line {}",
            line_num
        );
        assert!(
            val["timestamp"].is_string(),
            "Missing 'timestamp' field on line {}",
            line_num
        );
    }
}

// --- Tour pipeline integration test ---

#[test]
fn tour_processes_large_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let output_dir = dir.path().join("tour-output");

    let config = TourConfig::new(fixture_path()).with_output_dir(&output_dir);
    let result = vifei_tour::run_tour(&config).expect("Tour must process large fixture");

    // Verify metrics
    assert!(
        result.metrics.event_count_total >= 10_000,
        "Tour must report >= 10K committed events, got {}",
        result.metrics.event_count_total
    );
    assert_eq!(result.metrics.tier_a_drops, 0, "No Tier A drops expected");

    // Verify all proof artifacts exist
    assert!(output_dir.join("metrics.json").exists());
    assert!(output_dir.join("viewmodel.hash").exists());
    assert!(output_dir.join("ansi.capture").exists());
    assert!(output_dir.join("timetravel.capture").exists());

    // Verify viewmodel hash is 64-char BLAKE3 hex
    assert_eq!(result.viewmodel_hash.len(), 64);

    // Verify metrics.json is valid
    let metrics_json = fs::read_to_string(output_dir.join("metrics.json")).unwrap();
    let metrics: TourMetrics = serde_json::from_str(&metrics_json).unwrap();
    assert_eq!(metrics.event_count_total, result.metrics.event_count_total);
}

#[test]
fn tour_large_fixture_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let out1 = dir.path().join("out1");
    let out2 = dir.path().join("out2");

    let config1 = TourConfig::new(fixture_path()).with_output_dir(&out1);
    let config2 = TourConfig::new(fixture_path()).with_output_dir(&out2);

    let r1 = vifei_tour::run_tour(&config1).unwrap();
    let r2 = vifei_tour::run_tour(&config2).unwrap();

    // Same fixture → identical viewmodel hash
    assert_eq!(r1.viewmodel_hash, r2.viewmodel_hash);

    // Same metrics
    let m1 = fs::read_to_string(out1.join("metrics.json")).unwrap();
    let m2 = fs::read_to_string(out2.join("metrics.json")).unwrap();
    assert_eq!(m1, m2);
}
