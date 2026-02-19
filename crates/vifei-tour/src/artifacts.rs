use crate::TourMetrics;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;
use vifei_core::projection::{ExportSafetyState, LadderLevel, ViewModel};

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

/// Emit all Tour proof artifacts to the output directory.
pub(crate) fn emit_artifacts(
    output_dir: &Path,
    metrics: &TourMetrics,
    viewmodel: &ViewModel,
    vm_hash: &str,
    event_count: usize,
    seek_points: Vec<SeekPoint>,
) -> io::Result<()> {
    // Write metrics.json
    let metrics_path = output_dir.join("metrics.json");
    let metrics_json = serde_json::to_string_pretty(metrics).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize metrics: {e}"),
        )
    })?;
    fs::write(&metrics_path, metrics_json)?;

    // Write viewmodel.hash (single line, newline-terminated)
    let hash_path = output_dir.join("viewmodel.hash");
    fs::write(&hash_path, format!("{}\n", vm_hash))?;

    // Write ansi.capture — deterministic ANSI rendering of ViewModel state
    let ansi_path = output_dir.join("ansi.capture");
    let ansi_content = render_ansi_capture(viewmodel, event_count, vm_hash);
    fs::write(&ansi_path, &ansi_content)?;

    // Write timetravel.capture with ordered seek points
    let timetravel = TimeTravelCapture {
        projection_invariants_version: viewmodel.projection_invariants_version.clone(),
        seek_points,
    };
    let timetravel_path = output_dir.join("timetravel.capture");
    let timetravel_json = serde_json::to_string_pretty(&timetravel).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize timetravel: {e}"),
        )
    })?;
    fs::write(&timetravel_path, timetravel_json)?;

    Ok(())
}

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
        "{FG_MAGENTA}{BOLD}║  Vifei Tour · ansi.capture                             ║{RESET}"
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
