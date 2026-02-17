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

use panopticon_core::eventlog::EventLogWriter;
use panopticon_core::projection::{project, viewmodel_hash, ProjectionInvariants};
use panopticon_core::reducer::{reduce, state_hash, State};
use panopticon_import::cassette::parse_cassette;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufReader, Cursor};
use std::path::PathBuf;

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

/// Metrics emitted by Tour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourMetrics {
    /// Projection invariants version.
    pub projection_invariants_version: String,
    /// Total number of events processed.
    pub event_count_total: usize,
    /// Tier A drops (must be 0 for CI pass).
    pub tier_a_drops: u64,
    /// Maximum degradation level reached.
    pub max_degradation_level: String,
    /// Final degradation level.
    pub degradation_level_final: String,
    /// Degradation transitions (ordered list).
    pub degradation_transitions: Vec<DegradationTransition>,
    /// Aggregation mode.
    pub aggregation_mode: String,
    /// Aggregation bin size (if applicable).
    pub aggregation_bin_size: Option<u64>,
    /// Queue pressure (normalized 0.0-1.0).
    pub queue_pressure: f64,
    /// Export safety state.
    pub export_safety_state: String,
}

/// A degradation level transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationTransition {
    /// Level before transition.
    pub from_level: String,
    /// Level after transition.
    pub to_level: String,
    /// What triggered the transition.
    pub trigger: String,
    /// Queue pressure at transition time.
    pub queue_pressure: f64,
}

/// Run the Tour stress harness.
pub fn run_tour(config: &TourConfig) -> io::Result<TourResult> {
    // Validate stress mode is enabled
    if !config.stress {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Tour requires --stress flag in v0.1",
        ));
    }

    // Stage 1: Parse fixture
    let fixture_content = fs::read_to_string(&config.fixture_path)?;
    let reader = BufReader::new(Cursor::new(&fixture_content));
    let events = parse_cassette(reader);

    let event_count = events.len();
    if event_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Fixture contains no events",
        ));
    }

    // Create output directory
    fs::create_dir_all(&config.output_dir)?;

    // Stage 2: Import through append writer (to temp EventLog)
    let temp_dir = tempfile::tempdir()?;
    let eventlog_path = temp_dir.path().join("eventlog.jsonl");
    let mut writer = EventLogWriter::open(&eventlog_path)?;

    for event in events {
        writer.append(event)?;
    }
    drop(writer);

    // Stage 3: Reduce all events
    let mut state = State::new();
    let committed_events = panopticon_core::eventlog::read_eventlog(&eventlog_path)?;
    for event in &committed_events {
        state = reduce(&state, event);
    }

    // Stage 4: Project with stress-configured invariants
    let invariants = ProjectionInvariants::new();
    let viewmodel = project(&state, &invariants);

    // Stage 5: Build metrics
    let metrics = TourMetrics {
        projection_invariants_version: viewmodel.projection_invariants_version.clone(),
        event_count_total: event_count,
        tier_a_drops: viewmodel.tier_a_drops,
        max_degradation_level: format!("{}", viewmodel.degradation_level),
        degradation_level_final: format!("{}", viewmodel.degradation_level),
        degradation_transitions: Vec::new(), // No transitions in simple pipeline
        aggregation_mode: viewmodel.aggregation_mode.clone(),
        aggregation_bin_size: viewmodel.aggregation_bin_size,
        queue_pressure: viewmodel.queue_pressure(),
        export_safety_state: format!("{}", viewmodel.export_safety_state),
    };

    // Stage 6: Emit proof artifacts
    let vm_hash = viewmodel_hash(&viewmodel);

    // Write metrics.json
    let metrics_path = config.output_dir.join("metrics.json");
    let metrics_json = serde_json::to_string_pretty(&metrics).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize metrics: {e}"),
        )
    })?;
    fs::write(&metrics_path, metrics_json)?;

    // Write viewmodel.hash (single line, newline-terminated)
    let hash_path = config.output_dir.join("viewmodel.hash");
    fs::write(&hash_path, format!("{}\n", vm_hash))?;

    // Write placeholder for ansi.capture (M7.5)
    let ansi_path = config.output_dir.join("ansi.capture");
    fs::write(
        &ansi_path,
        "# ansi.capture placeholder\n# Full implementation in M7.5\n",
    )?;

    // Write timetravel.capture (M7.3)
    let timetravel = TimeTravelCapture {
        projection_invariants_version: viewmodel.projection_invariants_version.clone(),
        seek_points: vec![SeekPoint {
            commit_index: event_count.saturating_sub(1) as u64,
            state_hash: state_hash(&state),
            viewmodel_hash: vm_hash.clone(),
        }],
    };
    let timetravel_path = config.output_dir.join("timetravel.capture");
    let timetravel_json = serde_json::to_string_pretty(&timetravel).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize timetravel: {e}"),
        )
    })?;
    fs::write(&timetravel_path, timetravel_json)?;

    Ok(TourResult {
        output_dir: config.output_dir.clone(),
        metrics,
        viewmodel_hash: vm_hash,
    })
}

/// Time-travel capture artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelCapture {
    /// Projection invariants version.
    pub projection_invariants_version: String,
    /// Ordered list of seek points.
    pub seek_points: Vec<SeekPoint>,
}

/// A seek point in the time-travel capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekPoint {
    /// Commit index at this point.
    pub commit_index: u64,
    /// State hash at this point.
    pub state_hash: String,
    /// ViewModel hash at this point.
    pub viewmodel_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn create_fixture(dir: &Path) -> PathBuf {
        let fixture_path = dir.join("test.jsonl");
        let content = r#"{"type":"session_start","session_id":"test-1","timestamp":"2026-01-01T00:00:00Z","agent":"test"}
{"type":"tool_use","session_id":"test-1","timestamp":"2026-01-01T00:00:01Z","tool":"Read","id":"t1","args":{}}
{"type":"tool_result","session_id":"test-1","timestamp":"2026-01-01T00:00:02Z","tool":"Read","id":"t1","result":"ok"}
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
}
