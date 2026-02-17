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
use panopticon_core::projection::{
    project, viewmodel_hash, ExportSafetyState, LadderLevel, ProjectionInvariants, ViewModel,
};
use panopticon_core::reducer::{reduce, state_hash, State};
use panopticon_import::cassette::parse_cassette;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
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

    let imported_event_count = events.len();
    if imported_event_count == 0 {
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

    // Stage 3: Reduce all events with periodic seek point capture
    let mut state = State::new();
    let committed_events = panopticon_core::eventlog::read_eventlog(&eventlog_path)?;
    let committed_event_count = committed_events.len();

    // Capture ~20 seek points for time-travel replay, minimum 1 per event for small fixtures
    let seek_interval = (committed_event_count / 20).max(1);
    let mut seek_points = Vec::new();

    for (i, event) in committed_events.iter().enumerate() {
        state = reduce(&state, event);

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

    // Stage 4: Project final state
    let invariants = ProjectionInvariants::new();
    let viewmodel = project(&state, &invariants);

    // Stage 5: Build metrics
    // Populate degradation_transitions from reducer's policy_decisions
    let degradation_transitions: Vec<DegradationTransition> = state
        .policy_decisions
        .iter()
        .map(|pd| DegradationTransition {
            from_level: pd.from_level.clone(),
            to_level: pd.to_level.clone(),
            trigger: pd.trigger.clone(),
            queue_pressure: pd.queue_pressure_micro as f64 / 1_000_000.0,
        })
        .collect();

    // Compute max degradation level from transitions + final level
    let final_level = format!("{}", viewmodel.degradation_level);
    let max_degradation_level = state
        .policy_decisions
        .iter()
        .map(|pd| pd.to_level.as_str())
        .chain(std::iter::once(final_level.as_str()))
        .max()
        .unwrap_or("L0")
        .to_string();

    let metrics = TourMetrics {
        projection_invariants_version: viewmodel.projection_invariants_version.clone(),
        event_count_total: committed_event_count,
        tier_a_drops: viewmodel.tier_a_drops,
        max_degradation_level,
        degradation_level_final: final_level,
        degradation_transitions,
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

    // Write ansi.capture — deterministic ANSI rendering of ViewModel state
    let ansi_path = config.output_dir.join("ansi.capture");
    let ansi_content = render_ansi_capture(&viewmodel, committed_event_count, &vm_hash);
    fs::write(&ansi_path, &ansi_content)?;

    // Write timetravel.capture with ordered seek points
    let timetravel = TimeTravelCapture {
        projection_invariants_version: viewmodel.projection_invariants_version.clone(),
        seek_points,
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

// --- ANSI escape helpers (deterministic, no external dependencies) ---

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const FG_GREEN: &str = "\x1b[32m";
const FG_YELLOW: &str = "\x1b[33m";
const FG_RED: &str = "\x1b[31m";
const FG_WHITE: &str = "\x1b[37m";
const FG_MAGENTA: &str = "\x1b[35m";
const FG_GRAY: &str = "\x1b[90m";

/// ANSI color for degradation level (mirrors Truth HUD semantics).
fn ansi_level(level: LadderLevel) -> &'static str {
    match level {
        LadderLevel::L0 => FG_GREEN,
        LadderLevel::L1 | LadderLevel::L2 | LadderLevel::L3 => FG_YELLOW,
        LadderLevel::L4 | LadderLevel::L5 => FG_RED,
    }
}

/// ANSI color for Tier A drops.
fn ansi_drops(drops: u64) -> &'static str {
    if drops > 0 {
        FG_RED
    } else {
        FG_GREEN
    }
}

/// ANSI color for export safety state.
fn ansi_export(state: ExportSafetyState) -> &'static str {
    match state {
        ExportSafetyState::Unknown => FG_GRAY,
        ExportSafetyState::Clean => FG_GREEN,
        ExportSafetyState::Dirty | ExportSafetyState::Refused => FG_RED,
    }
}

/// ANSI color for queue pressure percentage.
fn ansi_pressure(pct: u32) -> &'static str {
    if pct >= 80 {
        FG_RED
    } else if pct >= 50 {
        FG_YELLOW
    } else {
        FG_GREEN
    }
}

/// Render deterministic ANSI capture of the ViewModel.
///
/// Mirrors Truth HUD layout and color semantics using raw ANSI escape codes.
/// Same ViewModel → identical output bytes (no wall-clock, no randomness).
fn render_ansi_capture(vm: &ViewModel, event_count: usize, vm_hash: &str) -> String {
    let mut buf = String::new();

    // Header
    let _ = writeln!(
        buf,
        "{FG_MAGENTA}{BOLD}╔══════════════════════════════════════════════════════════════╗{RESET}"
    );
    let _ = writeln!(
        buf,
        "{FG_MAGENTA}{BOLD}║  Panopticon Tour · ansi.capture                             ║{RESET}"
    );
    let _ = writeln!(
        buf,
        "{FG_MAGENTA}{BOLD}╚══════════════════════════════════════════════════════════════╝{RESET}"
    );
    let _ = writeln!(buf);

    // Truth HUD section
    let _ = writeln!(buf, "{FG_MAGENTA}{BOLD}── Truth HUD ──{RESET}");

    let level_color = ansi_level(vm.degradation_level);
    let _ = writeln!(
        buf,
        "  {FG_WHITE}Level:{RESET}    {level_color}{}{RESET}",
        vm.degradation_level,
    );

    let agg_display = vm
        .aggregation_bin_size
        .map(|bin| format!("{} (bin={bin})", vm.aggregation_mode))
        .unwrap_or_else(|| vm.aggregation_mode.clone());
    let _ = writeln!(buf, "  {FG_WHITE}Agg:{RESET}      {agg_display}");

    let pressure_pct = (vm.queue_pressure() * 100.0) as u32;
    let pressure_color = ansi_pressure(pressure_pct);
    let _ = writeln!(
        buf,
        "  {FG_WHITE}Pressure:{RESET} {pressure_color}{pressure_pct}%{RESET}",
    );

    let drops_color = ansi_drops(vm.tier_a_drops);
    let _ = writeln!(
        buf,
        "  {FG_WHITE}Drops:{RESET}    {drops_color}{}{RESET}",
        vm.tier_a_drops,
    );

    let export_color = ansi_export(vm.export_safety_state);
    let _ = writeln!(
        buf,
        "  {FG_WHITE}Export:{RESET}   {export_color}{}{RESET}",
        vm.export_safety_state,
    );

    let _ = writeln!(
        buf,
        "  {FG_GRAY}Version:{RESET}  {FG_GRAY}{}{RESET}",
        vm.projection_invariants_version,
    );

    let _ = writeln!(buf);

    // Summary section
    let _ = writeln!(buf, "{FG_MAGENTA}{BOLD}── Summary ──{RESET}");
    let _ = writeln!(buf, "  {FG_WHITE}Events:{RESET}   {event_count}");
    let _ = writeln!(buf, "  {FG_WHITE}Hash:{RESET}     {vm_hash}");

    buf
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
