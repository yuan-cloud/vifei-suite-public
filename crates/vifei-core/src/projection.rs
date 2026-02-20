//! Projection plus viewmodel.hash v0.1 — deterministic State → ViewModel.
//!
//! # Overview
//!
//! The projection is a pure function `(State, ProjectionInvariants) -> ViewModel`
//! that transforms reducer state into a renderable view model. The ViewModel
//! is what the TUI renders.
//!
//! # Purity contract
//!
//! - No IO.
//! - No randomness.
//! - No wall clock reads.
//! - No terminal size or focus state in hashed output.
//! - Same inputs always produce the same output.
//!
//! # Determinism strategy
//!
//! All map-like containers are [`BTreeMap`] (never `HashMap`).
//! `queue_pressure` is the only float; it uses explicit precision formatting for hashing.
//!
//! # viewmodel.hash
//!
//! `viewmodel.hash = BLAKE3(projection_invariants_version + canonical_serialize(ViewModel))`
//!
//! INCLUDE list (all ViewModel fields):
//! - `tier_a_summaries` (BTreeMap)
//! - `aggregation_mode` (String)
//! - `aggregation_bin_size` (Option<u64>)
//! - `degradation_level` (LadderLevel)
//! - `queue_pressure_fixed` (i64, quantized from f64 for determinism)
//! - `tier_a_drops` (u64)
//! - `export_safety_state` (ExportSafetyState)
//! - `projection_invariants_version` (String)
//!
//! EXCLUDE list (UI-only, not truth):
//! - terminal_size
//! - focus_state
//! - cursor_blink
//! - wall clock / timestamps
//! - random values
//!
//! # Invariants enforced
//!
//! - **I2 (Deterministic projection):** ViewModel is deterministic given State + invariants.
//! - **I4 (Testable determinism):** `viewmodel.hash` stability across runs.
//!
//! # Constitution
//!
//! See `docs/BACKPRESSURE_POLICY.md`:
//! - "Projection invariants v0.1" — honesty mechanics rules.
//! - "Degradation ladder" — L0 through L5 definitions.

use crate::reducer::State;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Constants (M5.1)
// ---------------------------------------------------------------------------

/// Projection invariants version from `docs/BACKPRESSURE_POLICY.md` § Versioning.
///
/// This version changes when:
/// - Any projection invariant rule is added, removed, or modified.
/// - The ViewModel include/exclude list for hashing changes.
///
/// Embedded in ViewModel, `metrics.json`, and `timetravel.capture`.
pub const PROJECTION_INVARIANTS_VERSION: &str = "projection-invariants-v0.1";

// ---------------------------------------------------------------------------
// LadderLevel (M5.1)
// ---------------------------------------------------------------------------

/// Degradation ladder level from `docs/BACKPRESSURE_POLICY.md`.
///
/// The ladder is the only allowed order of degradation. Escalation moves
/// one level at a time (L0 → L1 → L2 → L3 → L4), except fatal storage
/// failures which transition directly to L5.
///
/// # Ordering
///
/// `L0 < L1 < L2 < L3 < L4 < L5` — lower levels are healthier.
///
/// # Display
///
/// Displays as "L0", "L1", etc. to match BACKPRESSURE_POLICY identifiers.
///
/// # Constitution
///
/// Canonical definitions are in `docs/BACKPRESSURE_POLICY.md` § Degradation ladder.
/// Do NOT duplicate the prose definitions here — link to the constitution doc.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum LadderLevel {
    /// L0: Normal — 1:1 events rendered.
    #[default]
    L0,
    /// L1: Aggregate — Bin and summarize Tier B/C. Tier A remains 1:1.
    L1,
    /// L2: Collapse — Collapse Tier B/C into counts/histograms. Tier A remains 1:1.
    L2,
    /// L3: Reduce Fidelity — Fewer redraws, simplified rendering. Tier A remains 1:1.
    L3,
    /// L4: Freeze UI — Freeze non-HUD panes. Continue ingesting Tier A.
    L4,
    /// L5: Safe failure posture — Stop ingest. Keep last known-good truth readable.
    L5,
}

impl LadderLevel {
    /// All ladder levels in order from healthiest to most degraded.
    pub const ALL: [LadderLevel; 6] = [
        LadderLevel::L0,
        LadderLevel::L1,
        LadderLevel::L2,
        LadderLevel::L3,
        LadderLevel::L4,
        LadderLevel::L5,
    ];

    /// Returns true if this level represents normal operation.
    pub fn is_normal(&self) -> bool {
        *self == LadderLevel::L0
    }

    /// Returns true if this level represents safe failure posture.
    pub fn is_safe_failure(&self) -> bool {
        *self == LadderLevel::L5
    }

    /// Returns true if UI panes should be frozen at this level.
    pub fn is_ui_frozen(&self) -> bool {
        *self >= LadderLevel::L4
    }

    /// Returns true if Tier B/C events should be aggregated at this level.
    pub fn should_aggregate(&self) -> bool {
        *self >= LadderLevel::L1
    }

    /// Returns true if Tier B/C events should be collapsed to counts at this level.
    pub fn should_collapse(&self) -> bool {
        *self >= LadderLevel::L2
    }

    /// Returns the next escalation level, if any.
    /// L5 has no further escalation.
    pub fn escalate(&self) -> Option<LadderLevel> {
        match self {
            LadderLevel::L0 => Some(LadderLevel::L1),
            LadderLevel::L1 => Some(LadderLevel::L2),
            LadderLevel::L2 => Some(LadderLevel::L3),
            LadderLevel::L3 => Some(LadderLevel::L4),
            LadderLevel::L4 => Some(LadderLevel::L5),
            LadderLevel::L5 => None,
        }
    }

    /// Returns the next de-escalation level, if any.
    /// L0 has no further de-escalation.
    pub fn deescalate(&self) -> Option<LadderLevel> {
        match self {
            LadderLevel::L0 => None,
            LadderLevel::L1 => Some(LadderLevel::L0),
            LadderLevel::L2 => Some(LadderLevel::L1),
            LadderLevel::L3 => Some(LadderLevel::L2),
            LadderLevel::L4 => Some(LadderLevel::L3),
            LadderLevel::L5 => Some(LadderLevel::L4),
        }
    }
}

impl fmt::Display for LadderLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LadderLevel::L0 => write!(f, "L0"),
            LadderLevel::L1 => write!(f, "L1"),
            LadderLevel::L2 => write!(f, "L2"),
            LadderLevel::L3 => write!(f, "L3"),
            LadderLevel::L4 => write!(f, "L4"),
            LadderLevel::L5 => write!(f, "L5"),
        }
    }
}

/// Error returned when parsing an invalid ladder level string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLadderLevelError {
    /// The invalid input string.
    pub input: String,
}

impl fmt::Display for ParseLadderLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid ladder level '{}': expected L0, L1, L2, L3, L4, or L5",
            self.input
        )
    }
}

impl std::error::Error for ParseLadderLevelError {}

impl FromStr for LadderLevel {
    type Err = ParseLadderLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "L0" => Ok(LadderLevel::L0),
            "L1" => Ok(LadderLevel::L1),
            "L2" => Ok(LadderLevel::L2),
            "L3" => Ok(LadderLevel::L3),
            "L4" => Ok(LadderLevel::L4),
            "L5" => Ok(LadderLevel::L5),
            _ => Err(ParseLadderLevelError {
                input: s.to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// ProjectionInvariants (M5.1)
// ---------------------------------------------------------------------------

/// Parameters that control the projection function.
///
/// This struct captures the projection configuration that affects the
/// ViewModel output. It is NOT part of State (reducer output). It is
/// a separate input to the projection function, keeping the reducer
/// pure and the projection independently configurable.
///
/// # Fields
///
/// - `version`: The projection invariants version string. Changes when
///   invariant rules change. Embedded in ViewModel and proof artifacts.
/// - `degradation_level`: Current position on the degradation ladder.
///   Controls how Tier B/C events are rendered (aggregated, collapsed, etc.).
///
/// # Constitution
///
/// See `docs/BACKPRESSURE_POLICY.md` § "Projection invariants v0.1" and
/// § "Degradation ladder" for the authoritative definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionInvariants {
    /// Projection invariants version from `docs/BACKPRESSURE_POLICY.md`.
    /// Default: [`PROJECTION_INVARIANTS_VERSION`].
    pub version: String,

    /// Current degradation ladder level.
    /// Default: [`LadderLevel::L0`] (normal operation).
    pub degradation_level: LadderLevel,
}

impl ProjectionInvariants {
    /// Create projection invariants with default version and L0 (normal).
    pub fn new() -> Self {
        ProjectionInvariants {
            version: PROJECTION_INVARIANTS_VERSION.to_string(),
            degradation_level: LadderLevel::L0,
        }
    }

    /// Create projection invariants with a specific degradation level.
    pub fn with_level(level: LadderLevel) -> Self {
        ProjectionInvariants {
            version: PROJECTION_INVARIANTS_VERSION.to_string(),
            degradation_level: level,
        }
    }

    /// Returns true if operating at normal level.
    pub fn is_normal(&self) -> bool {
        self.degradation_level.is_normal()
    }

    /// Returns true if in safe failure posture.
    pub fn is_safe_failure(&self) -> bool {
        self.degradation_level.is_safe_failure()
    }
}

impl Default for ProjectionInvariants {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ExportSafetyState (M5.2)
// ---------------------------------------------------------------------------

/// Export safety state for the Truth HUD.
///
/// Indicates whether the EventLog is safe to share externally. Until
/// M8 (share-safe export) is implemented, the state remains [`Unknown`].
///
/// # Constitution
///
/// See `PLANS.md` § "Truth HUD" for the authoritative state definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExportSafetyState {
    /// Export safety has not been evaluated. Default until M8 export scan.
    #[default]
    Unknown,
    /// No secrets detected. Safe to export.
    Clean,
    /// Secrets detected. Export would be unsafe.
    Dirty,
    /// Export was attempted but refused due to secrets.
    Refused,
}

impl ExportSafetyState {
    /// All export safety states.
    pub const ALL: [ExportSafetyState; 4] = [
        ExportSafetyState::Unknown,
        ExportSafetyState::Clean,
        ExportSafetyState::Dirty,
        ExportSafetyState::Refused,
    ];

    /// Returns true if the state indicates safety has not been evaluated.
    pub fn is_unknown(&self) -> bool {
        *self == ExportSafetyState::Unknown
    }

    /// Returns true if the EventLog is safe to export.
    pub fn is_safe(&self) -> bool {
        *self == ExportSafetyState::Clean
    }

    /// Returns true if secrets were detected.
    pub fn has_secrets(&self) -> bool {
        matches!(self, ExportSafetyState::Dirty | ExportSafetyState::Refused)
    }
}

impl fmt::Display for ExportSafetyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportSafetyState::Unknown => write!(f, "UNKNOWN"),
            ExportSafetyState::Clean => write!(f, "CLEAN"),
            ExportSafetyState::Dirty => write!(f, "DIRTY"),
            ExportSafetyState::Refused => write!(f, "REFUSED"),
        }
    }
}

/// Error returned when parsing an invalid export safety state string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseExportSafetyStateError {
    /// The invalid input string.
    pub input: String,
}

impl fmt::Display for ParseExportSafetyStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid export safety state '{}': expected UNKNOWN, CLEAN, DIRTY, or REFUSED",
            self.input
        )
    }
}

impl std::error::Error for ParseExportSafetyStateError {}

impl FromStr for ExportSafetyState {
    type Err = ParseExportSafetyStateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "UNKNOWN" => Ok(ExportSafetyState::Unknown),
            "CLEAN" => Ok(ExportSafetyState::Clean),
            "DIRTY" => Ok(ExportSafetyState::Dirty),
            "REFUSED" => Ok(ExportSafetyState::Refused),
            _ => Err(ParseExportSafetyStateError {
                input: s.to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// ViewModel (M5.2)
// ---------------------------------------------------------------------------

/// Precision for queue_pressure quantization.
/// Multiply f64 by this value and truncate to i64 for deterministic hashing.
/// 6 decimal places = 1,000,000.
pub(crate) const QUEUE_PRESSURE_PRECISION: i64 = 1_000_000;

/// The hashable data structure that drives the TUI.
///
/// ViewModel is the output of the projection function and the input to
/// rendering. It contains all the "confession" fields that the Truth HUD
/// must display, plus any additional state needed for rendering.
///
/// # Determinism
///
/// - All map-like fields use [`BTreeMap`] (not `HashMap`).
/// - `queue_pressure` is stored as `queue_pressure_fixed` (i64) after
///   quantization to avoid float nondeterminism in hashing.
/// - Original `queue_pressure` f64 is available via [`Self::queue_pressure()`].
///
/// # Explicitly excluded (UI-only, not part of hash)
///
/// These are NOT in ViewModel because they are terminal/UI state:
/// - Terminal size / window dimensions
/// - Focus state (which pane is focused)
/// - Cursor blink state
/// - Wall clock / current time
/// - Random values
///
/// The TUI layer adds these at render time, outside the hash boundary.
///
/// # Constitution
///
/// See `PLANS.md` § "Truth HUD" and `docs/BACKPRESSURE_POLICY.md`
/// § "Projection invariants v0.1" for field requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewModel {
    /// Tier A event counts by type name (e.g., "RunStart" -> 5).
    /// Uses BTreeMap for deterministic ordering.
    pub tier_a_summaries: BTreeMap<String, u64>,

    /// Aggregation mode string, e.g., "1:1", "10:1", "collapsed".
    /// Describes how Tier B/C events are summarized at the current ladder level.
    pub aggregation_mode: String,

    /// Bin size when aggregating, or None for 1:1 mode.
    pub aggregation_bin_size: Option<u64>,

    /// Current degradation ladder level.
    pub degradation_level: LadderLevel,

    /// Queue pressure as fixed-point integer (f64 * QUEUE_PRESSURE_PRECISION).
    /// Use [`Self::queue_pressure()`] to get the f64 value.
    /// Stored as i64 for deterministic serialization and hashing.
    pub queue_pressure_fixed: i64,

    /// Number of Tier A events that were dropped.
    /// MUST be 0 in normal operation (invariant I1).
    pub tier_a_drops: u64,

    /// Export safety state for the Truth HUD.
    pub export_safety_state: ExportSafetyState,

    /// Projection invariants version embedded for traceability.
    pub projection_invariants_version: String,
}

impl ViewModel {
    /// Create a new ViewModel with default/empty values.
    pub fn new() -> Self {
        ViewModel {
            tier_a_summaries: BTreeMap::new(),
            aggregation_mode: "1:1".to_string(),
            aggregation_bin_size: None,
            degradation_level: LadderLevel::L0,
            queue_pressure_fixed: 0,
            tier_a_drops: 0,
            export_safety_state: ExportSafetyState::Unknown,
            projection_invariants_version: PROJECTION_INVARIANTS_VERSION.to_string(),
        }
    }

    /// Get queue pressure as f64 in range [0.0, 1.0].
    pub fn queue_pressure(&self) -> f64 {
        self.queue_pressure_fixed as f64 / QUEUE_PRESSURE_PRECISION as f64
    }

    /// Set queue pressure from f64. Clamps to [0.0, 1.0] and quantizes.
    pub fn set_queue_pressure(&mut self, pressure: f64) {
        let clamped = pressure.clamp(0.0, 1.0);
        self.queue_pressure_fixed = (clamped * QUEUE_PRESSURE_PRECISION as f64) as i64;
    }

    /// Create queue_pressure_fixed from f64. Clamps to [0.0, 1.0] and quantizes.
    pub fn quantize_queue_pressure(pressure: f64) -> i64 {
        let clamped = pressure.clamp(0.0, 1.0);
        (clamped * QUEUE_PRESSURE_PRECISION as f64) as i64
    }

    /// Returns true if the system is in normal operation (L0, no drops).
    pub fn is_healthy(&self) -> bool {
        self.degradation_level.is_normal() && self.tier_a_drops == 0
    }

    /// Returns true if any Tier A events were dropped (invariant violation).
    pub fn has_tier_a_drops(&self) -> bool {
        self.tier_a_drops > 0
    }

    /// Returns true if the UI should be frozen at the current degradation level.
    pub fn is_ui_frozen(&self) -> bool {
        self.degradation_level.is_ui_frozen()
    }
}

impl Default for ViewModel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// project() function (M5.3)
// ---------------------------------------------------------------------------

/// Deterministic projection function: State + ProjectionInvariants → ViewModel.
///
/// This is a pure function with no IO, no randomness, and no wall clock reads.
/// Same inputs will always produce the same output.
///
/// # Purity contract
///
/// - No IO.
/// - No randomness.
/// - No wall clock reads.
/// - No terminal size or focus state.
/// - All iteration is by `commit_index`, never by timestamp.
///
/// # Projection invariants (from BACKPRESSURE_POLICY)
///
/// - NEVER fabricate events.
/// - NEVER reorder truth.
/// - May summarize Tier B/C per ladder level, but MUST confess in Truth HUD.
///
/// # Ladder level behavior
///
/// - L0 (Normal): Full detail, 1:1 events.
/// - L1 (Aggregate): Aggregation mode "10:1" with bin size.
/// - L2 (Collapse): Aggregation mode "collapsed", counts only.
/// - L3 (Reduce Fidelity): Same as L2 for data, UI may simplify rendering.
/// - L4 (Freeze UI): Same as L3, UI frozen except Truth HUD.
/// - L5 (Safe Failure): Minimal data, failure state indication.
pub fn project(state: &State, invariants: &ProjectionInvariants) -> ViewModel {
    // Build tier_a_summaries from event_counts_by_type, filtering for Tier A types.
    // Tier A event types from PLANS.md D2: RunStart, RunEnd, ToolCall, ToolResult,
    // PolicyDecision, RedactionApplied, Error, ClockSkewDetected.
    let tier_a_types = [
        "RunStart",
        "RunEnd",
        "ToolCall",
        "ToolResult",
        "PolicyDecision",
        "RedactionApplied",
        "Error",
        "ClockSkewDetected",
    ];

    let mut tier_a_summaries = BTreeMap::new();
    for type_name in &tier_a_types {
        if let Some(&count) = state.event_counts_by_type.get(*type_name) {
            if count > 0 {
                tier_a_summaries.insert(type_name.to_string(), count);
            }
        }
    }

    // Determine aggregation mode based on degradation level.
    let (aggregation_mode, aggregation_bin_size) = match invariants.degradation_level {
        LadderLevel::L0 => ("1:1".to_string(), None),
        LadderLevel::L1 => ("10:1".to_string(), Some(10)),
        LadderLevel::L2 | LadderLevel::L3 | LadderLevel::L4 => ("collapsed".to_string(), None),
        LadderLevel::L5 => ("frozen".to_string(), None),
    };

    // Get queue pressure from the last policy decision, if any.
    // queue_pressure_micro is stored as millionths (0..=1_000_000).
    let queue_pressure_fixed = state
        .policy_decisions
        .last()
        .map(|pd| pd.queue_pressure_micro as i64)
        .unwrap_or(0);

    ViewModel {
        tier_a_summaries,
        aggregation_mode,
        aggregation_bin_size,
        degradation_level: invariants.degradation_level,
        queue_pressure_fixed,
        tier_a_drops: state.tier_a_drops,
        export_safety_state: ExportSafetyState::Unknown, // Until M8 export scan
        projection_invariants_version: invariants.version.clone(),
    }
}

/// Project with additional context for queue pressure.
///
/// Use this when you have a live queue pressure value from the backpressure
/// controller, rather than relying on the last recorded PolicyDecision event.
pub fn project_with_pressure(
    state: &State,
    invariants: &ProjectionInvariants,
    queue_pressure: f64,
) -> ViewModel {
    let mut vm = project(state, invariants);
    vm.set_queue_pressure(queue_pressure);
    vm
}

// ---------------------------------------------------------------------------
// viewmodel.hash computation (M5.4)
// ---------------------------------------------------------------------------

/// Compute `viewmodel.hash` — the BLAKE3 hash of a ViewModel.
///
/// # Formula
///
/// `viewmodel.hash = BLAKE3(canonical_serialize(ViewModel))`
///
/// # Include list (all fields affect the hash)
///
/// - `tier_a_summaries`: BTreeMap<String, u64>
/// - `aggregation_mode`: String
/// - `aggregation_bin_size`: Option<u64>
/// - `degradation_level`: LadderLevel
/// - `queue_pressure_fixed`: i64 (quantized from f64, avoids float nondeterminism)
/// - `tier_a_drops`: u64
/// - `export_safety_state`: ExportSafetyState
/// - `projection_invariants_version`: String
///
/// # Exclude list
///
/// Nothing is excluded in v0.1. All ViewModel fields are meaningful for
/// determinism verification.
///
/// # Float handling
///
/// `queue_pressure` is stored as `queue_pressure_fixed` (i64), which is
/// `queue_pressure * 1,000,000` truncated. This avoids platform-dependent
/// float formatting in the serialized output.
///
/// # Determinism
///
/// - ViewModel uses BTreeMap (not HashMap) for `tier_a_summaries`.
/// - Serialization uses `serde_json::to_vec` which produces deterministic
///   output for structs with stable field order.
/// - `projection_invariants_version` is embedded in the ViewModel, so
///   changes to projection rules produce different hashes.
///
/// # Output format
///
/// Returns a 64-character lowercase hex string (BLAKE3 produces 256-bit hash).
/// For file output, the caller should append a newline.
pub fn viewmodel_hash(vm: &ViewModel) -> String {
    // Serialize ViewModel to canonical JSON bytes.
    // ViewModel contains only primitive types and BTreeMaps, so serialization
    // should never fail.
    let bytes = serde_json::to_vec(vm).expect("ViewModel serialization should never fail");

    // Compute BLAKE3 hash
    let hash = blake3::hash(&bytes);

    // Return lowercase hex string
    hash.to_hex().to_string()
}

/// Compute viewmodel.hash and return as a newline-terminated string for file output.
///
/// This is the format expected for Tour proof artifacts (`viewmodel.hash` file).
pub fn viewmodel_hash_for_file(vm: &ViewModel) -> String {
    format!("{}\n", viewmodel_hash(vm))
}

// ---------------------------------------------------------------------------
// Tests (M5.1, M5.2, M5.3, M5.4)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // LadderLevel tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ladder_level_ordering() {
        // L0 < L1 < L2 < L3 < L4 < L5
        assert!(LadderLevel::L0 < LadderLevel::L1);
        assert!(LadderLevel::L1 < LadderLevel::L2);
        assert!(LadderLevel::L2 < LadderLevel::L3);
        assert!(LadderLevel::L3 < LadderLevel::L4);
        assert!(LadderLevel::L4 < LadderLevel::L5);
    }

    #[test]
    fn test_ladder_level_display() {
        assert_eq!(LadderLevel::L0.to_string(), "L0");
        assert_eq!(LadderLevel::L1.to_string(), "L1");
        assert_eq!(LadderLevel::L2.to_string(), "L2");
        assert_eq!(LadderLevel::L3.to_string(), "L3");
        assert_eq!(LadderLevel::L4.to_string(), "L4");
        assert_eq!(LadderLevel::L5.to_string(), "L5");
    }

    #[test]
    fn test_ladder_level_from_str() {
        assert_eq!("L0".parse::<LadderLevel>().unwrap(), LadderLevel::L0);
        assert_eq!("l1".parse::<LadderLevel>().unwrap(), LadderLevel::L1);
        assert_eq!("L2".parse::<LadderLevel>().unwrap(), LadderLevel::L2);
        assert_eq!("l3".parse::<LadderLevel>().unwrap(), LadderLevel::L3);
        assert_eq!("L4".parse::<LadderLevel>().unwrap(), LadderLevel::L4);
        assert_eq!("l5".parse::<LadderLevel>().unwrap(), LadderLevel::L5);

        // Invalid inputs
        assert!("L6".parse::<LadderLevel>().is_err());
        assert!("invalid".parse::<LadderLevel>().is_err());
        assert!("".parse::<LadderLevel>().is_err());
    }

    #[test]
    fn test_ladder_level_serialize_json() {
        // Serializes as uppercase string
        assert_eq!(serde_json::to_string(&LadderLevel::L0).unwrap(), "\"L0\"");
        assert_eq!(serde_json::to_string(&LadderLevel::L5).unwrap(), "\"L5\"");
    }

    #[test]
    fn test_ladder_level_deserialize_json() {
        assert_eq!(
            serde_json::from_str::<LadderLevel>("\"L0\"").unwrap(),
            LadderLevel::L0
        );
        assert_eq!(
            serde_json::from_str::<LadderLevel>("\"L5\"").unwrap(),
            LadderLevel::L5
        );
    }

    #[test]
    fn test_ladder_level_round_trip() {
        for level in LadderLevel::ALL {
            let json = serde_json::to_string(&level).unwrap();
            let parsed: LadderLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, level);
        }
    }

    #[test]
    fn test_ladder_level_escalate() {
        assert_eq!(LadderLevel::L0.escalate(), Some(LadderLevel::L1));
        assert_eq!(LadderLevel::L1.escalate(), Some(LadderLevel::L2));
        assert_eq!(LadderLevel::L2.escalate(), Some(LadderLevel::L3));
        assert_eq!(LadderLevel::L3.escalate(), Some(LadderLevel::L4));
        assert_eq!(LadderLevel::L4.escalate(), Some(LadderLevel::L5));
        assert_eq!(LadderLevel::L5.escalate(), None);
    }

    #[test]
    fn test_ladder_level_deescalate() {
        assert_eq!(LadderLevel::L0.deescalate(), None);
        assert_eq!(LadderLevel::L1.deescalate(), Some(LadderLevel::L0));
        assert_eq!(LadderLevel::L2.deescalate(), Some(LadderLevel::L1));
        assert_eq!(LadderLevel::L3.deescalate(), Some(LadderLevel::L2));
        assert_eq!(LadderLevel::L4.deescalate(), Some(LadderLevel::L3));
        assert_eq!(LadderLevel::L5.deescalate(), Some(LadderLevel::L4));
    }

    #[test]
    fn test_ladder_level_predicates() {
        assert!(LadderLevel::L0.is_normal());
        assert!(!LadderLevel::L1.is_normal());

        assert!(LadderLevel::L5.is_safe_failure());
        assert!(!LadderLevel::L4.is_safe_failure());

        assert!(!LadderLevel::L3.is_ui_frozen());
        assert!(LadderLevel::L4.is_ui_frozen());
        assert!(LadderLevel::L5.is_ui_frozen());

        assert!(!LadderLevel::L0.should_aggregate());
        assert!(LadderLevel::L1.should_aggregate());

        assert!(!LadderLevel::L1.should_collapse());
        assert!(LadderLevel::L2.should_collapse());
    }

    #[test]
    fn test_ladder_level_default() {
        assert_eq!(LadderLevel::default(), LadderLevel::L0);
    }

    #[test]
    fn test_ladder_level_all_constant() {
        assert_eq!(LadderLevel::ALL.len(), 6);
        assert_eq!(LadderLevel::ALL[0], LadderLevel::L0);
        assert_eq!(LadderLevel::ALL[5], LadderLevel::L5);
    }

    // -----------------------------------------------------------------------
    // ProjectionInvariants tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_projection_invariants_new() {
        let inv = ProjectionInvariants::new();
        assert_eq!(inv.version, PROJECTION_INVARIANTS_VERSION);
        assert_eq!(inv.degradation_level, LadderLevel::L0);
    }

    #[test]
    fn test_projection_invariants_with_level() {
        let inv = ProjectionInvariants::with_level(LadderLevel::L3);
        assert_eq!(inv.version, PROJECTION_INVARIANTS_VERSION);
        assert_eq!(inv.degradation_level, LadderLevel::L3);
    }

    #[test]
    fn test_projection_invariants_default() {
        let inv = ProjectionInvariants::default();
        assert_eq!(inv.version, PROJECTION_INVARIANTS_VERSION);
        assert_eq!(inv.degradation_level, LadderLevel::L0);
    }

    #[test]
    fn test_projection_invariants_predicates() {
        let normal = ProjectionInvariants::new();
        assert!(normal.is_normal());
        assert!(!normal.is_safe_failure());

        let failed = ProjectionInvariants::with_level(LadderLevel::L5);
        assert!(!failed.is_normal());
        assert!(failed.is_safe_failure());
    }

    #[test]
    fn test_projection_invariants_serialize_json() {
        let inv = ProjectionInvariants::new();
        let json = serde_json::to_string(&inv).unwrap();
        assert!(json.contains("projection-invariants-v0.1"));
        assert!(json.contains("\"degradation_level\":\"L0\""));
    }

    #[test]
    fn test_projection_invariants_round_trip() {
        for level in LadderLevel::ALL {
            let inv = ProjectionInvariants::with_level(level);
            let json = serde_json::to_string(&inv).unwrap();
            let parsed: ProjectionInvariants = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, inv);
        }
    }

    #[test]
    fn test_projection_invariants_version_constant() {
        assert_eq!(PROJECTION_INVARIANTS_VERSION, "projection-invariants-v0.1");
    }

    // -----------------------------------------------------------------------
    // Determinism tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ladder_level_byte_stable_serialization() {
        // Same level should produce identical JSON bytes across runs
        let bytes1 = serde_json::to_vec(&LadderLevel::L3).unwrap();
        let bytes2 = serde_json::to_vec(&LadderLevel::L3).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_projection_invariants_byte_stable_serialization() {
        // Same invariants should produce identical JSON bytes across runs
        let inv = ProjectionInvariants::with_level(LadderLevel::L2);
        let bytes1 = serde_json::to_vec(&inv).unwrap();
        let bytes2 = serde_json::to_vec(&inv).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    // -----------------------------------------------------------------------
    // ExportSafetyState tests (M5.2)
    // -----------------------------------------------------------------------

    #[test]
    fn test_export_safety_state_default() {
        assert_eq!(ExportSafetyState::default(), ExportSafetyState::Unknown);
    }

    #[test]
    fn test_export_safety_state_display() {
        assert_eq!(ExportSafetyState::Unknown.to_string(), "UNKNOWN");
        assert_eq!(ExportSafetyState::Clean.to_string(), "CLEAN");
        assert_eq!(ExportSafetyState::Dirty.to_string(), "DIRTY");
        assert_eq!(ExportSafetyState::Refused.to_string(), "REFUSED");
    }

    #[test]
    fn test_export_safety_state_from_str() {
        assert_eq!(
            "UNKNOWN".parse::<ExportSafetyState>().unwrap(),
            ExportSafetyState::Unknown
        );
        assert_eq!(
            "clean".parse::<ExportSafetyState>().unwrap(),
            ExportSafetyState::Clean
        );
        assert_eq!(
            "Dirty".parse::<ExportSafetyState>().unwrap(),
            ExportSafetyState::Dirty
        );
        assert_eq!(
            "REFUSED".parse::<ExportSafetyState>().unwrap(),
            ExportSafetyState::Refused
        );

        // Invalid inputs
        assert!("invalid".parse::<ExportSafetyState>().is_err());
        assert!("".parse::<ExportSafetyState>().is_err());
    }

    #[test]
    fn test_export_safety_state_serialize_json() {
        assert_eq!(
            serde_json::to_string(&ExportSafetyState::Unknown).unwrap(),
            "\"UNKNOWN\""
        );
        assert_eq!(
            serde_json::to_string(&ExportSafetyState::Clean).unwrap(),
            "\"CLEAN\""
        );
        assert_eq!(
            serde_json::to_string(&ExportSafetyState::Dirty).unwrap(),
            "\"DIRTY\""
        );
        assert_eq!(
            serde_json::to_string(&ExportSafetyState::Refused).unwrap(),
            "\"REFUSED\""
        );
    }

    #[test]
    fn test_export_safety_state_round_trip() {
        for state in ExportSafetyState::ALL {
            let json = serde_json::to_string(&state).unwrap();
            let parsed: ExportSafetyState = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, state);
        }
    }

    #[test]
    fn test_export_safety_state_predicates() {
        assert!(ExportSafetyState::Unknown.is_unknown());
        assert!(!ExportSafetyState::Clean.is_unknown());

        assert!(ExportSafetyState::Clean.is_safe());
        assert!(!ExportSafetyState::Dirty.is_safe());
        assert!(!ExportSafetyState::Unknown.is_safe());

        assert!(ExportSafetyState::Dirty.has_secrets());
        assert!(ExportSafetyState::Refused.has_secrets());
        assert!(!ExportSafetyState::Clean.has_secrets());
        assert!(!ExportSafetyState::Unknown.has_secrets());
    }

    // -----------------------------------------------------------------------
    // ViewModel tests (M5.2)
    // -----------------------------------------------------------------------

    #[test]
    fn test_viewmodel_new() {
        let vm = ViewModel::new();
        assert!(vm.tier_a_summaries.is_empty());
        assert_eq!(vm.aggregation_mode, "1:1");
        assert_eq!(vm.aggregation_bin_size, None);
        assert_eq!(vm.degradation_level, LadderLevel::L0);
        assert_eq!(vm.queue_pressure_fixed, 0);
        assert_eq!(vm.tier_a_drops, 0);
        assert_eq!(vm.export_safety_state, ExportSafetyState::Unknown);
        assert_eq!(
            vm.projection_invariants_version,
            PROJECTION_INVARIANTS_VERSION
        );
    }

    #[test]
    fn test_viewmodel_default() {
        let vm = ViewModel::default();
        assert_eq!(vm, ViewModel::new());
    }

    #[test]
    fn test_viewmodel_queue_pressure_roundtrip() {
        let mut vm = ViewModel::new();

        // Test various pressure values
        for pressure in [0.0, 0.5, 0.123456, 0.999999, 1.0] {
            vm.set_queue_pressure(pressure);
            let recovered = vm.queue_pressure();
            // Should be equal within quantization precision
            assert!(
                (recovered - pressure).abs() < 1e-6,
                "pressure {} recovered as {}",
                pressure,
                recovered
            );
        }
    }

    #[test]
    fn test_viewmodel_queue_pressure_clamp() {
        let mut vm = ViewModel::new();

        // Values below 0 should clamp to 0
        vm.set_queue_pressure(-0.5);
        assert_eq!(vm.queue_pressure(), 0.0);

        // Values above 1 should clamp to 1
        vm.set_queue_pressure(1.5);
        assert_eq!(vm.queue_pressure(), 1.0);
    }

    #[test]
    fn test_viewmodel_quantize_queue_pressure() {
        assert_eq!(ViewModel::quantize_queue_pressure(0.0), 0);
        assert_eq!(ViewModel::quantize_queue_pressure(0.5), 500_000);
        assert_eq!(ViewModel::quantize_queue_pressure(1.0), 1_000_000);

        // Clamping
        assert_eq!(ViewModel::quantize_queue_pressure(-1.0), 0);
        assert_eq!(ViewModel::quantize_queue_pressure(2.0), 1_000_000);
    }

    #[test]
    fn test_viewmodel_is_healthy() {
        let mut vm = ViewModel::new();
        assert!(vm.is_healthy()); // L0, no drops

        vm.degradation_level = LadderLevel::L1;
        assert!(!vm.is_healthy()); // Not L0

        vm.degradation_level = LadderLevel::L0;
        vm.tier_a_drops = 1;
        assert!(!vm.is_healthy()); // Has drops
    }

    #[test]
    fn test_viewmodel_has_tier_a_drops() {
        let mut vm = ViewModel::new();
        assert!(!vm.has_tier_a_drops());

        vm.tier_a_drops = 1;
        assert!(vm.has_tier_a_drops());
    }

    #[test]
    fn test_viewmodel_is_ui_frozen() {
        let mut vm = ViewModel::new();
        assert!(!vm.is_ui_frozen()); // L0

        vm.degradation_level = LadderLevel::L3;
        assert!(!vm.is_ui_frozen()); // L3

        vm.degradation_level = LadderLevel::L4;
        assert!(vm.is_ui_frozen()); // L4

        vm.degradation_level = LadderLevel::L5;
        assert!(vm.is_ui_frozen()); // L5
    }

    #[test]
    fn test_viewmodel_serialize_json() {
        let mut vm = ViewModel::new();
        vm.tier_a_summaries.insert("RunStart".to_string(), 1);
        vm.tier_a_summaries.insert("ToolCall".to_string(), 5);
        vm.set_queue_pressure(0.75);

        let json = serde_json::to_string(&vm).unwrap();

        // Check key fields are present
        assert!(json.contains("\"tier_a_summaries\""));
        assert!(json.contains("\"RunStart\":1"));
        assert!(json.contains("\"ToolCall\":5"));
        assert!(json.contains("\"aggregation_mode\":\"1:1\""));
        assert!(json.contains("\"degradation_level\":\"L0\""));
        assert!(json.contains("\"queue_pressure_fixed\":750000"));
        assert!(json.contains("\"tier_a_drops\":0"));
        assert!(json.contains("\"export_safety_state\":\"UNKNOWN\""));
        assert!(json.contains("\"projection_invariants_version\":\"projection-invariants-v0.1\""));
    }

    #[test]
    fn test_viewmodel_round_trip() {
        let mut vm = ViewModel::new();
        vm.tier_a_summaries.insert("RunStart".to_string(), 3);
        vm.tier_a_summaries.insert("Error".to_string(), 1);
        vm.aggregation_mode = "10:1".to_string();
        vm.aggregation_bin_size = Some(10);
        vm.degradation_level = LadderLevel::L2;
        vm.set_queue_pressure(0.42);
        vm.tier_a_drops = 0;
        vm.export_safety_state = ExportSafetyState::Clean;

        let json = serde_json::to_string(&vm).unwrap();
        let parsed: ViewModel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, vm);
    }

    #[test]
    fn test_viewmodel_byte_stable_serialization() {
        let mut vm = ViewModel::new();
        vm.tier_a_summaries.insert("A".to_string(), 1);
        vm.tier_a_summaries.insert("B".to_string(), 2);
        vm.set_queue_pressure(0.5);

        // Same ViewModel should produce identical bytes
        let bytes1 = serde_json::to_vec(&vm).unwrap();
        let bytes2 = serde_json::to_vec(&vm).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_viewmodel_btreemap_ordering() {
        // Keys should be sorted alphabetically in serialization
        let mut vm = ViewModel::new();
        vm.tier_a_summaries.insert("Zebra".to_string(), 1);
        vm.tier_a_summaries.insert("Apple".to_string(), 2);
        vm.tier_a_summaries.insert("Mango".to_string(), 3);

        let json = serde_json::to_string(&vm).unwrap();

        // BTreeMap should serialize with sorted keys
        let apple_pos = json.find("\"Apple\"").unwrap();
        let mango_pos = json.find("\"Mango\"").unwrap();
        let zebra_pos = json.find("\"Zebra\"").unwrap();

        assert!(apple_pos < mango_pos);
        assert!(mango_pos < zebra_pos);
    }

    #[test]
    fn test_viewmodel_no_excluded_fields() {
        // Verify ViewModel does NOT have excluded fields
        // This is a compile-time check via the struct definition,
        // but we verify the JSON output doesn't contain them
        let vm = ViewModel::new();
        let json = serde_json::to_string(&vm).unwrap();

        assert!(!json.contains("terminal_size"));
        assert!(!json.contains("focus_state"));
        assert!(!json.contains("cursor_blink"));
        assert!(!json.contains("wall_clock"));
        assert!(!json.contains("current_time"));
    }

    #[test]
    fn test_queue_pressure_precision_constant() {
        assert_eq!(QUEUE_PRESSURE_PRECISION, 1_000_000);
    }

    // -----------------------------------------------------------------------
    // project() function tests (M5.3)
    // -----------------------------------------------------------------------

    use crate::reducer::{PolicyTransition, State};

    #[test]
    fn test_project_empty_state() {
        let state = State::new();
        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        assert!(vm.tier_a_summaries.is_empty());
        assert_eq!(vm.aggregation_mode, "1:1");
        assert_eq!(vm.aggregation_bin_size, None);
        assert_eq!(vm.degradation_level, LadderLevel::L0);
        assert_eq!(vm.queue_pressure_fixed, 0);
        assert_eq!(vm.tier_a_drops, 0);
        assert_eq!(vm.export_safety_state, ExportSafetyState::Unknown);
        assert_eq!(
            vm.projection_invariants_version,
            PROJECTION_INVARIANTS_VERSION
        );
    }

    #[test]
    fn test_project_with_events() {
        let mut state = State::new();
        state.event_counts_by_type.insert("RunStart".to_string(), 1);
        state.event_counts_by_type.insert("ToolCall".to_string(), 5);
        state.event_counts_by_type.insert("Error".to_string(), 2);
        state.event_counts_by_type.insert("Generic".to_string(), 10); // Not Tier A
        state.tier_a_drops = 0;

        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        // Only Tier A types should be in summaries
        assert_eq!(vm.tier_a_summaries.get("RunStart"), Some(&1));
        assert_eq!(vm.tier_a_summaries.get("ToolCall"), Some(&5));
        assert_eq!(vm.tier_a_summaries.get("Error"), Some(&2));
        assert_eq!(vm.tier_a_summaries.get("Generic"), None); // Not Tier A
    }

    #[test]
    fn test_project_ladder_level_l0() {
        let state = State::new();
        let invariants = ProjectionInvariants::with_level(LadderLevel::L0);
        let vm = project(&state, &invariants);

        assert_eq!(vm.aggregation_mode, "1:1");
        assert_eq!(vm.aggregation_bin_size, None);
        assert_eq!(vm.degradation_level, LadderLevel::L0);
    }

    #[test]
    fn test_project_ladder_level_l1() {
        let state = State::new();
        let invariants = ProjectionInvariants::with_level(LadderLevel::L1);
        let vm = project(&state, &invariants);

        assert_eq!(vm.aggregation_mode, "10:1");
        assert_eq!(vm.aggregation_bin_size, Some(10));
        assert_eq!(vm.degradation_level, LadderLevel::L1);
    }

    #[test]
    fn test_project_ladder_level_l2() {
        let state = State::new();
        let invariants = ProjectionInvariants::with_level(LadderLevel::L2);
        let vm = project(&state, &invariants);

        assert_eq!(vm.aggregation_mode, "collapsed");
        assert_eq!(vm.aggregation_bin_size, None);
        assert_eq!(vm.degradation_level, LadderLevel::L2);
    }

    #[test]
    fn test_project_ladder_level_l5() {
        let state = State::new();
        let invariants = ProjectionInvariants::with_level(LadderLevel::L5);
        let vm = project(&state, &invariants);

        assert_eq!(vm.aggregation_mode, "frozen");
        assert_eq!(vm.aggregation_bin_size, None);
        assert_eq!(vm.degradation_level, LadderLevel::L5);
    }

    #[test]
    fn test_project_with_policy_decision() {
        let mut state = State::new();
        state.policy_decisions.push(PolicyTransition {
            commit_index: 100,
            from_level: "L0".to_string(),
            to_level: "L1".to_string(),
            trigger: "queue_pressure".to_string(),
            queue_pressure_micro: 800_000, // 0.8
        });

        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        assert_eq!(vm.queue_pressure_fixed, 800_000);
        assert!((vm.queue_pressure() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_project_with_tier_a_drops() {
        let mut state = State::new();
        state.tier_a_drops = 5;

        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        assert_eq!(vm.tier_a_drops, 5);
        assert!(vm.has_tier_a_drops());
        assert!(!vm.is_healthy());
    }

    #[test]
    fn test_project_with_pressure() {
        let state = State::new();
        let invariants = ProjectionInvariants::new();
        let vm = project_with_pressure(&state, &invariants, 0.65);

        assert!((vm.queue_pressure() - 0.65).abs() < 1e-6);
    }

    #[test]
    fn test_project_determinism() {
        // Same inputs should always produce the same output
        let mut state = State::new();
        state.event_counts_by_type.insert("RunStart".to_string(), 1);
        state.event_counts_by_type.insert("ToolCall".to_string(), 3);
        state.policy_decisions.push(PolicyTransition {
            commit_index: 50,
            from_level: "L0".to_string(),
            to_level: "L1".to_string(),
            trigger: "test".to_string(),
            queue_pressure_micro: 500_000,
        });

        let invariants = ProjectionInvariants::with_level(LadderLevel::L1);

        // Run projection 10 times
        let first = project(&state, &invariants);
        for _ in 0..10 {
            let vm = project(&state, &invariants);
            assert_eq!(vm, first, "Projection must be deterministic");
        }
    }

    #[test]
    fn test_project_byte_stable_serialization() {
        let mut state = State::new();
        state.event_counts_by_type.insert("RunStart".to_string(), 2);
        state.event_counts_by_type.insert("Error".to_string(), 1);

        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        let bytes1 = serde_json::to_vec(&vm).unwrap();
        let bytes2 = serde_json::to_vec(&vm).unwrap();
        assert_eq!(bytes1, bytes2, "Serialization must be byte-stable");
    }

    #[test]
    fn test_project_invariants_version_embedded() {
        let state = State::new();
        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        assert_eq!(vm.projection_invariants_version, invariants.version);
        assert_eq!(
            vm.projection_invariants_version,
            PROJECTION_INVARIANTS_VERSION
        );
    }

    #[test]
    fn test_project_all_tier_a_types() {
        let mut state = State::new();
        // Add all Tier A types
        state.event_counts_by_type.insert("RunStart".to_string(), 1);
        state.event_counts_by_type.insert("RunEnd".to_string(), 1);
        state.event_counts_by_type.insert("ToolCall".to_string(), 5);
        state
            .event_counts_by_type
            .insert("ToolResult".to_string(), 5);
        state
            .event_counts_by_type
            .insert("PolicyDecision".to_string(), 2);
        state
            .event_counts_by_type
            .insert("RedactionApplied".to_string(), 1);
        state.event_counts_by_type.insert("Error".to_string(), 3);
        state
            .event_counts_by_type
            .insert("ClockSkewDetected".to_string(), 1);

        let invariants = ProjectionInvariants::new();
        let vm = project(&state, &invariants);

        assert_eq!(vm.tier_a_summaries.len(), 8);
        assert_eq!(vm.tier_a_summaries.get("RunStart"), Some(&1));
        assert_eq!(vm.tier_a_summaries.get("RunEnd"), Some(&1));
        assert_eq!(vm.tier_a_summaries.get("ToolCall"), Some(&5));
        assert_eq!(vm.tier_a_summaries.get("ToolResult"), Some(&5));
        assert_eq!(vm.tier_a_summaries.get("PolicyDecision"), Some(&2));
        assert_eq!(vm.tier_a_summaries.get("RedactionApplied"), Some(&1));
        assert_eq!(vm.tier_a_summaries.get("Error"), Some(&3));
        assert_eq!(vm.tier_a_summaries.get("ClockSkewDetected"), Some(&1));
    }

    // -----------------------------------------------------------------------
    // viewmodel_hash tests (M5.4)
    // -----------------------------------------------------------------------

    #[test]
    fn test_viewmodel_hash_format() {
        let vm = ViewModel::new();
        let hash = viewmodel_hash(&vm);

        // BLAKE3 produces 256-bit (32-byte) hash, which is 64 hex characters
        assert_eq!(hash.len(), 64);

        // Should be lowercase hex
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash, hash.to_lowercase());
    }

    #[test]
    fn test_viewmodel_hash_for_file_format() {
        let vm = ViewModel::new();
        let file_content = viewmodel_hash_for_file(&vm);

        // Should end with newline
        assert!(file_content.ends_with('\n'));

        // Should be 65 characters (64 hex + 1 newline)
        assert_eq!(file_content.len(), 65);

        // Trimmed should be the hash
        let hash = viewmodel_hash(&vm);
        assert_eq!(file_content.trim(), hash);
    }

    #[test]
    fn test_viewmodel_hash_determinism() {
        let mut vm = ViewModel::new();
        vm.tier_a_summaries.insert("RunStart".to_string(), 3);
        vm.tier_a_summaries.insert("ToolCall".to_string(), 10);
        vm.set_queue_pressure(0.75);
        vm.degradation_level = LadderLevel::L2;

        // Same ViewModel should produce identical hash across runs
        let hash1 = viewmodel_hash(&vm);
        let hash2 = viewmodel_hash(&vm);
        assert_eq!(hash1, hash2);

        // Run 10 times to verify stability
        for _ in 0..10 {
            assert_eq!(viewmodel_hash(&vm), hash1);
        }
    }

    #[test]
    fn test_viewmodel_hash_changes_with_content() {
        let vm1 = ViewModel::new();

        let mut vm2 = ViewModel::new();
        vm2.tier_a_summaries.insert("Error".to_string(), 1);

        // Different ViewModels should produce different hashes
        assert_ne!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_changes_with_degradation_level() {
        let mut vm1 = ViewModel::new();
        vm1.degradation_level = LadderLevel::L0;

        let mut vm2 = ViewModel::new();
        vm2.degradation_level = LadderLevel::L1;

        assert_ne!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_changes_with_queue_pressure() {
        let mut vm1 = ViewModel::new();
        vm1.set_queue_pressure(0.5);

        let mut vm2 = ViewModel::new();
        vm2.set_queue_pressure(0.6);

        assert_ne!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_changes_with_tier_a_drops() {
        let vm1 = ViewModel::new(); // tier_a_drops = 0

        let mut vm2 = ViewModel::new();
        vm2.tier_a_drops = 1;

        assert_ne!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_changes_with_export_safety_state() {
        let mut vm1 = ViewModel::new();
        vm1.export_safety_state = ExportSafetyState::Unknown;

        let mut vm2 = ViewModel::new();
        vm2.export_safety_state = ExportSafetyState::Clean;

        assert_ne!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_includes_version() {
        let mut vm1 = ViewModel::new();
        vm1.projection_invariants_version = "projection-invariants-v0.1".to_string();

        let mut vm2 = ViewModel::new();
        vm2.projection_invariants_version = "projection-invariants-v0.2".to_string();

        // Different versions should produce different hashes
        assert_ne!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_btreemap_order_independence() {
        // Insert keys in different orders, but BTreeMap should sort them
        let mut vm1 = ViewModel::new();
        vm1.tier_a_summaries.insert("Zebra".to_string(), 1);
        vm1.tier_a_summaries.insert("Apple".to_string(), 2);
        vm1.tier_a_summaries.insert("Mango".to_string(), 3);

        let mut vm2 = ViewModel::new();
        vm2.tier_a_summaries.insert("Apple".to_string(), 2);
        vm2.tier_a_summaries.insert("Mango".to_string(), 3);
        vm2.tier_a_summaries.insert("Zebra".to_string(), 1);

        // Same data should produce same hash regardless of insertion order
        assert_eq!(viewmodel_hash(&vm1), viewmodel_hash(&vm2));
    }

    #[test]
    fn test_viewmodel_hash_empty_vs_nonempty() {
        let empty = ViewModel::new();

        let mut nonempty = ViewModel::new();
        nonempty.tier_a_summaries.insert("RunStart".to_string(), 1);

        assert_ne!(viewmodel_hash(&empty), viewmodel_hash(&nonempty));
    }

    #[test]
    fn test_viewmodel_hash_quantized_pressure_stability() {
        // Verify that quantized pressure produces stable hashes
        let mut vm = ViewModel::new();

        // Set pressure multiple times to same value
        vm.set_queue_pressure(0.123456);
        let hash1 = viewmodel_hash(&vm);

        vm.set_queue_pressure(0.123456);
        let hash2 = viewmodel_hash(&vm);

        assert_eq!(hash1, hash2);
    }

    // -----------------------------------------------------------------------
    // Full pipeline stability test (M5.5)
    // -----------------------------------------------------------------------

    /// M5.5: Same EventLog + same ProjectionInvariants → same viewmodel.hash across 10 runs.
    ///
    /// This tests the full pipeline: events → reduce → project → hash.
    /// Catches: HashMap regressions, float instability, any nondeterminism.
    #[test]
    fn test_full_pipeline_determinism_10_runs() {
        use crate::event::{CommittedEvent, EventPayload, ImportEvent, Tier};
        use crate::reducer::{reduce, State};

        // Create a deterministic set of events
        let events = vec![
            CommittedEvent::commit(
                ImportEvent {
                    run_id: "run-001".to_string(),
                    event_id: "evt-001".to_string(),
                    source_id: "test-source".to_string(),
                    source_seq: Some(0),
                    timestamp_ns: 1000000000,
                    tier: Tier::A,
                    payload: EventPayload::RunStart {
                        agent: "test-agent".to_string(),
                        args: Some("--test".to_string()),
                    },
                    payload_ref: None,
                    synthesized: false,
                },
                0,
            ),
            CommittedEvent::commit(
                ImportEvent {
                    run_id: "run-001".to_string(),
                    event_id: "evt-002".to_string(),
                    source_id: "test-source".to_string(),
                    source_seq: Some(1),
                    timestamp_ns: 2000000000,
                    tier: Tier::A,
                    payload: EventPayload::ToolCall {
                        tool: "grep".to_string(),
                        args: Some("pattern file.txt".to_string()),
                    },
                    payload_ref: None,
                    synthesized: false,
                },
                1,
            ),
            CommittedEvent::commit(
                ImportEvent {
                    run_id: "run-001".to_string(),
                    event_id: "evt-003".to_string(),
                    source_id: "test-source".to_string(),
                    source_seq: Some(2),
                    timestamp_ns: 3000000000,
                    tier: Tier::A,
                    payload: EventPayload::ToolResult {
                        tool: "grep".to_string(),
                        result: Some("match found".to_string()),
                        status: Some("success".to_string()),
                    },
                    payload_ref: None,
                    synthesized: false,
                },
                2,
            ),
            CommittedEvent::commit(
                ImportEvent {
                    run_id: "run-001".to_string(),
                    event_id: "evt-004".to_string(),
                    source_id: "test-source".to_string(),
                    source_seq: Some(3),
                    timestamp_ns: 4000000000,
                    tier: Tier::A,
                    payload: EventPayload::RunEnd {
                        exit_code: Some(0),
                        reason: Some("completed".to_string()),
                    },
                    payload_ref: None,
                    synthesized: false,
                },
                3,
            ),
        ];

        let invariants = ProjectionInvariants::new();
        let mut hashes = Vec::new();

        // Run the full pipeline 10 times
        for _ in 0..10 {
            // Reduce events to state
            let mut state = State::new();
            for event in &events {
                state = reduce(&state, event);
            }

            // Project state to ViewModel
            let vm = project(&state, &invariants);

            // Compute hash
            let hash = viewmodel_hash(&vm);
            hashes.push(hash);
        }

        // All 10 hashes must be identical
        let first = &hashes[0];
        for (i, hash) in hashes.iter().enumerate() {
            assert_eq!(
                hash, first,
                "Hash mismatch at run {}: expected '{}', got '{}'",
                i, first, hash
            );
        }
    }

    /// M5.5 variant: Test stability across different ladder levels.
    #[test]
    fn test_full_pipeline_determinism_all_ladder_levels() {
        use crate::event::{CommittedEvent, EventPayload, ImportEvent, Tier};
        use crate::reducer::{reduce, State};

        let event = CommittedEvent::commit(
            ImportEvent {
                run_id: "run-001".to_string(),
                event_id: "evt-001".to_string(),
                source_id: "test-source".to_string(),
                source_seq: Some(0),
                timestamp_ns: 1000000000,
                tier: Tier::A,
                payload: EventPayload::RunStart {
                    agent: "test-agent".to_string(),
                    args: None,
                },
                payload_ref: None,
                synthesized: false,
            },
            0,
        );

        // Test each ladder level
        for level in LadderLevel::ALL {
            let invariants = ProjectionInvariants::with_level(level);
            let mut hashes = Vec::new();

            for _ in 0..5 {
                let mut state = State::new();
                state = reduce(&state, &event);
                let vm = project(&state, &invariants);
                hashes.push(viewmodel_hash(&vm));
            }

            let first = &hashes[0];
            for hash in &hashes {
                assert_eq!(
                    hash, first,
                    "Hash instability at {:?}: hashes differ across runs",
                    level
                );
            }
        }
    }
}
