//! Tour stress harness — deterministic proof artifact generator.
//!
//! # Overview
//!
//! Tour is NOT a benchmark. It is a deterministic stress harness that proves
//! under load, truth was not compromised. Same fixture → same artifacts, always.
//!
//! # Pipeline
//!
//! ```text
//! fixture → import → append → reduce → project → emit artifacts
//! ```
//!
//! # Proof artifacts (emitted to tour-output/)
//!
//! | Artifact | Format | Purpose |
//! |---|---|---|
//! | `metrics.json` | JSON | Timing, throughput, drop counts, queue depths |
//! | `viewmodel.hash` | Plain text | Determinism proof (BLAKE3 hex, newline-terminated) |
//! | `ansi.capture` | ANSI text | Visual regression baseline |
//! | `timetravel.capture` | JSON | Time-travel replay artifact |
//!
//! # Determinism invariants
//!
//! - No random seeds
//! - No wall-clock-dependent behavior in artifact content
//! - No platform-dependent formatting
//! - Same fixture → identical artifacts
//!
//! # Benchmarking
//!
//! For local stage-level latency baselines without external crates:
//!
//! ```text
//! VIFEI_TOUR_BENCH_ITERS=10 cargo run -p vifei-tour --bin bench_tour --release
//! ```

mod artifacts;
mod metrics;

use artifacts::emit_artifacts;
pub use artifacts::{SeekPoint, TimeTravelCapture};
use metrics::build_metrics;
pub use metrics::{DegradationTransition, TourMetrics};
use std::fs;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use vifei_core::eventlog::EventLogWriter;
use vifei_core::projection::{project, viewmodel_hash, ProjectionInvariants};
use vifei_core::reducer::{reduce_in_place, state_hash, State};
use vifei_import::cassette::parse_cassette;

/// Tour configuration.
#[derive(Debug, Clone)]
pub struct TourConfig {
    /// Path to the fixture file (Agent Cassette JSONL).
    pub fixture_path: PathBuf,
    /// Output directory for proof artifacts.
    pub output_dir: PathBuf,
    /// Enable stress mode (required for v0.1).
    pub stress: bool,
}

impl TourConfig {
    /// Create a new Tour configuration.
    pub fn new(fixture_path: impl Into<PathBuf>) -> Self {
        TourConfig {
            fixture_path: fixture_path.into(),
            output_dir: PathBuf::from("tour-output"),
            stress: true,
        }
    }

    /// Set the output directory.
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }
}

/// Result of a Tour run.
#[derive(Debug)]
pub struct TourResult {
    /// Path to the output directory.
    pub output_dir: PathBuf,
    /// The emitted metrics.
    pub metrics: TourMetrics,
    /// The viewmodel hash.
    pub viewmodel_hash: String,
}

/// Stage-level timing profile for a Tour run.
#[derive(Debug, Clone)]
pub struct TourStageProfile {
    pub parse_fixture: Duration,
    pub append_writer: Duration,
    pub reducer: Duration,
    pub projection: Duration,
    pub metrics_emit: Duration,
    pub total: Duration,
}

/// Run the Tour stress harness.
pub fn run_tour(config: &TourConfig) -> io::Result<TourResult> {
    let (result, _) = run_tour_with_profile(config)?;
    Ok(result)
}

/// Run the Tour stress harness and return stage-level timing profile.
pub fn run_tour_with_profile(config: &TourConfig) -> io::Result<(TourResult, TourStageProfile)> {
    // Validate stress mode is enabled
    if !config.stress {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Tour requires --stress flag in v0.1",
        ));
    }
    let total_start = Instant::now();

    // Stage 1: Parse fixture
    let parse_start = Instant::now();
    let fixture_file = fs::File::open(&config.fixture_path)?;
    let reader = BufReader::new(fixture_file);
    let events = parse_cassette(reader);
    let parse_fixture = parse_start.elapsed();

    let imported_event_count = events.len();
    if imported_event_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Fixture contains no events",
        ));
    }

    // Create output directory
    fs::create_dir_all(&config.output_dir)?;

    // Stage 2: Import through append writer (to temp EventLog), while collecting
    // the exact committed sequence from append results.
    let append_start = Instant::now();
    let temp_dir = tempfile::tempdir()?;
    let eventlog_path = temp_dir.path().join("eventlog.jsonl");
    let mut writer = EventLogWriter::open(&eventlog_path)?;
    let mut committed_events = Vec::with_capacity(imported_event_count * 2);

    for event in events {
        let result = writer.append(event)?;
        committed_events.extend(result.detection_events().iter().cloned());
        committed_events.push(result.committed_event().clone());
    }
    drop(writer);
    let append_writer = append_start.elapsed();

    // Stage 3: Reduce all events with periodic seek point capture
    let reducer_start = Instant::now();
    let mut state = State::new();
    let committed_event_count = committed_events.len();

    // Capture ~20 seek points for time-travel replay, minimum 1 per event for small fixtures
    let seek_interval = (committed_event_count / 20).max(1);
    let mut seek_points = Vec::new();

    for (i, event) in committed_events.iter().enumerate() {
        reduce_in_place(&mut state, event);

        let is_interval = (i + 1) % seek_interval == 0;
        let is_last = i == committed_event_count - 1;
        if is_interval || is_last {
            let inv = ProjectionInvariants::new();
            let vm = project(&state, &inv);
            seek_points.push(SeekPoint {
                commit_index: event.commit_index,
                state_hash: state_hash(&state),
                viewmodel_hash: viewmodel_hash(&vm),
            });
        }
    }
    let reducer = reducer_start.elapsed();

    // Stage 4: Project final state
    let projection_start = Instant::now();
    let invariants = ProjectionInvariants::new();
    let viewmodel = project(&state, &invariants);
    let projection = projection_start.elapsed();

    // Stage 5: Build metrics
    let metrics_start = Instant::now();
    let metrics = build_metrics(&state, &viewmodel, committed_event_count);

    // Stage 6: Emit proof artifacts
    let vm_hash = viewmodel_hash(&viewmodel);
    emit_artifacts(
        &config.output_dir,
        &metrics,
        &viewmodel,
        &vm_hash,
        committed_event_count,
        seek_points,
    )?;
    let metrics_emit = metrics_start.elapsed();
    let total = total_start.elapsed();

    let result = TourResult {
        output_dir: config.output_dir.clone(),
        metrics,
        viewmodel_hash: vm_hash,
    };
    let profile = TourStageProfile {
        parse_fixture,
        append_writer,
        reducer,
        projection,
        metrics_emit,
        total,
    };

    Ok((result, profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{BufReader, Cursor};
    use std::path::Path;
    use tempfile::tempdir;
    use vifei_core::eventlog::{read_eventlog, EventLogWriter};

    fn create_fixture(dir: &Path) -> PathBuf {
        let fixture_path = dir.join("test.jsonl");
        let content = r#"{"type":"session_start","session_id":"test-1","timestamp":"2026-01-01T00:00:00Z","agent":"test"}
{"type":"tool_use","session_id":"test-1","timestamp":"2026-01-01T00:00:01Z","tool":"Read","id":"t1","args":{}}
{"type":"tool_result","session_id":"test-1","timestamp":"2026-01-01T00:00:02Z","tool":"Read","id":"t1","result":"ok"}
{"type":"session_end","session_id":"test-1","timestamp":"2026-01-01T00:00:03Z"}"#;
        fs::write(&fixture_path, content).unwrap();
        fixture_path
    }

    fn create_clock_skew_fixture(dir: &Path) -> PathBuf {
        let fixture_path = dir.join("clock-skew.jsonl");
        // Third event timestamp moves backward by 1s (well above 50ms tolerance),
        // so append writer should inject a ClockSkewDetected event.
        let content = r#"{"type":"session_start","session_id":"test-1","timestamp":"2026-01-01T00:00:00Z","agent":"test"}
{"type":"tool_use","session_id":"test-1","timestamp":"2026-01-01T00:00:02Z","tool":"Read","id":"t1","args":{}}
{"type":"tool_result","session_id":"test-1","timestamp":"2026-01-01T00:00:01Z","tool":"Read","id":"t1","result":"ok"}
{"type":"session_end","session_id":"test-1","timestamp":"2026-01-01T00:00:03Z"}"#;
        fs::write(&fixture_path, content).unwrap();
        fixture_path
    }

    #[test]
    fn tour_config_defaults() {
        let config = TourConfig::new("fixture.jsonl");
        assert_eq!(config.fixture_path, PathBuf::from("fixture.jsonl"));
        assert_eq!(config.output_dir, PathBuf::from("tour-output"));
        assert!(config.stress);
    }

    #[test]
    fn tour_config_with_output_dir() {
        let config = TourConfig::new("fixture.jsonl").with_output_dir("custom-output");
        assert_eq!(config.output_dir, PathBuf::from("custom-output"));
    }

    #[test]
    fn run_tour_requires_stress_flag() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());

        let mut config = TourConfig::new(&fixture_path);
        config.stress = false;

        let result = run_tour(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--stress"));
    }

    #[test]
    fn run_tour_empty_fixture_fails() {
        let dir = tempdir().unwrap();
        let fixture_path = dir.path().join("empty.jsonl");
        fs::write(&fixture_path, "").unwrap();

        let config = TourConfig::new(&fixture_path).with_output_dir(dir.path().join("output"));

        let result = run_tour(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no events"));
    }

    #[test]
    fn run_tour_produces_artifacts() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        let result = run_tour(&config).unwrap();

        // Check artifacts exist
        assert!(output_dir.join("metrics.json").exists());
        assert!(output_dir.join("viewmodel.hash").exists());
        assert!(output_dir.join("ansi.capture").exists());
        assert!(output_dir.join("timetravel.capture").exists());

        // Check metrics content
        let metrics_content = fs::read_to_string(output_dir.join("metrics.json")).unwrap();
        let metrics: TourMetrics = serde_json::from_str(&metrics_content).unwrap();
        assert_eq!(metrics.event_count_total, 4);
        assert_eq!(metrics.tier_a_drops, 0);

        // Check viewmodel.hash format
        let hash_content = fs::read_to_string(output_dir.join("viewmodel.hash")).unwrap();
        assert_eq!(hash_content.trim().len(), 64); // BLAKE3 hex
        assert!(hash_content.ends_with('\n'));

        // Check result
        assert_eq!(result.metrics.event_count_total, 4);
        assert_eq!(result.viewmodel_hash.len(), 64);
    }

    #[test]
    fn artifact_serialization_policy_pretty_json_surfaces() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        run_tour(&config).unwrap();

        let metrics_json = fs::read_to_string(output_dir.join("metrics.json")).unwrap();
        let timetravel_json = fs::read_to_string(output_dir.join("timetravel.capture")).unwrap();

        // Policy: these JSON artifacts are pretty-serialized and newline-indented.
        assert!(
            metrics_json.starts_with("{\n  \""),
            "metrics.json must be pretty JSON with two-space indentation"
        );
        assert!(
            timetravel_json.starts_with("{\n  \""),
            "timetravel.capture must be pretty JSON with two-space indentation"
        );
    }

    #[test]
    fn run_tour_determinism() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());

        // Run Tour twice with different output dirs
        let output1 = dir.path().join("output1");
        let output2 = dir.path().join("output2");

        let config1 = TourConfig::new(&fixture_path).with_output_dir(&output1);
        let config2 = TourConfig::new(&fixture_path).with_output_dir(&output2);

        let result1 = run_tour(&config1).unwrap();
        let result2 = run_tour(&config2).unwrap();

        // Same fixture → same viewmodel.hash
        assert_eq!(result1.viewmodel_hash, result2.viewmodel_hash);

        // Same metrics
        let metrics1 = fs::read_to_string(output1.join("metrics.json")).unwrap();
        let metrics2 = fs::read_to_string(output2.join("metrics.json")).unwrap();
        assert_eq!(metrics1, metrics2);

        // Same ansi.capture
        let ansi1 = fs::read_to_string(output1.join("ansi.capture")).unwrap();
        let ansi2 = fs::read_to_string(output2.join("ansi.capture")).unwrap();
        assert_eq!(ansi1, ansi2);
    }

    #[test]
    fn append_result_sequence_matches_eventlog_readback() {
        let dir = tempdir().unwrap();
        let fixture_path = create_clock_skew_fixture(dir.path());
        let fixture_content = fs::read_to_string(&fixture_path).unwrap();
        let events = parse_cassette(BufReader::new(Cursor::new(fixture_content)));

        let eventlog_path = dir.path().join("eventlog.jsonl");
        let mut writer = EventLogWriter::open(&eventlog_path).unwrap();
        let mut from_append = Vec::new();

        for event in events {
            let result = writer.append(event).unwrap();
            from_append.extend(result.detection_events().iter().cloned());
            from_append.push(result.committed_event().clone());
        }
        drop(writer);

        let from_readback = read_eventlog(&eventlog_path).unwrap();
        assert_eq!(
            from_append, from_readback,
            "append results must preserve canonical committed sequence"
        );
    }

    #[test]
    fn stream_fixture_parse_matches_buffered_parse() {
        let dir = tempdir().unwrap();
        let fixture_path = create_clock_skew_fixture(dir.path());
        let fixture_content = fs::read_to_string(&fixture_path).unwrap();

        let buffered = parse_cassette(BufReader::new(Cursor::new(fixture_content)));
        let streamed = parse_cassette(BufReader::new(fs::File::open(&fixture_path).unwrap()));

        assert_eq!(streamed, buffered);
    }

    #[test]
    fn timetravel_capture_format() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        run_tour(&config).unwrap();

        let content = fs::read_to_string(output_dir.join("timetravel.capture")).unwrap();
        let capture: TimeTravelCapture = serde_json::from_str(&content).unwrap();

        assert!(!capture.projection_invariants_version.is_empty());
        assert!(!capture.seek_points.is_empty());

        let point = &capture.seek_points[0];
        assert_eq!(point.state_hash.len(), 64);
        assert_eq!(point.viewmodel_hash.len(), 64);
    }

    #[test]
    fn event_count_and_commit_index_use_committed_events() {
        let dir = tempdir().unwrap();
        let fixture_path = create_clock_skew_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        let result = run_tour(&config).unwrap();

        // 4 imported events + 1 synthesized ClockSkewDetected event
        assert_eq!(result.metrics.event_count_total, 5);

        let timetravel_content = fs::read_to_string(output_dir.join("timetravel.capture")).unwrap();
        let capture: TimeTravelCapture = serde_json::from_str(&timetravel_content).unwrap();
        // 5 committed events with seek_interval=max(5/20,1)=1 → seek point per event
        assert_eq!(capture.seek_points.len(), 5);
        // Last seek point must reference the final commit index
        assert_eq!(capture.seek_points.last().unwrap().commit_index, 4);
    }

    #[test]
    fn seek_points_monotonically_ordered() {
        let dir = tempdir().unwrap();
        let fixture_path = create_clock_skew_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        run_tour(&config).unwrap();

        let content = fs::read_to_string(output_dir.join("timetravel.capture")).unwrap();
        let capture: TimeTravelCapture = serde_json::from_str(&content).unwrap();

        // commit_index must be monotonically increasing across seek points
        for window in capture.seek_points.windows(2) {
            assert!(
                window[1].commit_index > window[0].commit_index,
                "Seek points not ordered: {} should be > {}",
                window[1].commit_index,
                window[0].commit_index,
            );
        }
    }

    #[test]
    fn metrics_schema_has_all_required_fields() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        run_tour(&config).unwrap();

        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output_dir.join("metrics.json")).unwrap())
                .unwrap();

        // All PLANS.md required fields must be present
        for key in &[
            "projection_invariants_version",
            "event_count_total",
            "tier_a_drops",
            "max_degradation_level",
            "degradation_level_final",
            "degradation_transitions",
            "aggregation_mode",
            "aggregation_bin_size",
            "queue_pressure",
            "export_safety_state",
        ] {
            assert!(
                raw.get(key).is_some(),
                "metrics.json missing required field: {}",
                key,
            );
        }

        // degradation_transitions must be an array
        assert!(raw["degradation_transitions"].is_array());
    }

    #[test]
    fn degradation_transitions_structure() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        let result = run_tour(&config).unwrap();

        // Small fixture has no backpressure transitions
        assert!(result.metrics.degradation_transitions.is_empty());

        // But degradation_level_final and max_degradation_level must be populated
        assert!(!result.metrics.degradation_level_final.is_empty());
        assert!(!result.metrics.max_degradation_level.is_empty());
    }

    #[test]
    fn ansi_capture_contains_truth_hud_fields() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        let result = run_tour(&config).unwrap();

        let ansi = fs::read_to_string(output_dir.join("ansi.capture")).unwrap();

        // All 6 Truth HUD fields must appear
        assert!(ansi.contains("Level:"), "Missing Level label");
        assert!(ansi.contains("L0"), "Missing level value");
        assert!(ansi.contains("Agg:"), "Missing Agg label");
        assert!(ansi.contains("Pressure:"), "Missing Pressure label");
        assert!(ansi.contains("Drops:"), "Missing Drops label");
        assert!(ansi.contains("Export:"), "Missing Export label");
        assert!(ansi.contains("Version:"), "Missing Version label");

        // Summary section
        assert!(ansi.contains("Events:"), "Missing event count");
        assert!(ansi.contains("Hash:"), "Missing hash");
        assert!(ansi.contains(&result.viewmodel_hash), "Hash value mismatch");
    }

    #[test]
    fn ansi_capture_contains_escape_codes() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        run_tour(&config).unwrap();

        let ansi = fs::read_to_string(output_dir.join("ansi.capture")).unwrap();

        // Must contain ANSI escape sequences (not plain text)
        assert!(ansi.contains("\x1b["), "No ANSI escape codes found");
        // Must contain reset sequences
        assert!(ansi.contains("\x1b[0m"), "No ANSI reset codes found");
    }

    #[test]
    fn ansi_capture_not_placeholder() {
        let dir = tempdir().unwrap();
        let fixture_path = create_fixture(dir.path());
        let output_dir = dir.path().join("output");

        let config = TourConfig::new(&fixture_path).with_output_dir(&output_dir);
        run_tour(&config).unwrap();

        let ansi = fs::read_to_string(output_dir.join("ansi.capture")).unwrap();

        // Must not be the old placeholder
        assert!(
            !ansi.contains("placeholder"),
            "ansi.capture still contains placeholder text"
        );
    }
}
