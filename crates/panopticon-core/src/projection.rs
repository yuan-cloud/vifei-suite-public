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
// Tests (M5.1)
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
}
