//! Reducer plus checkpoints v0.1 -- pure state machine for the EventLog.
//!
//! # Overview
//!
//! The reducer is a pure function `(State, CommittedEvent) -> State` that
//! rebuilds application state from the EventLog. It is the bridge between
//! raw truth (events) and usable data (state).
//!
//! # Purity contract
//!
//! - No IO.
//! - No randomness.
//! - No wall clock reads.
//! - No interior mutability that could leak nondeterminism.
//! - Same inputs always produce the same output.
//!
//! # Determinism strategy
//!
//! All map-like containers in [`State`] are [`BTreeMap`] (never `HashMap`).
//! No floats in State. All iteration is deterministic.
//!
//! # Checkpoint semantics
//!
//! Every 5000 events (from `docs/CAPACITY_ENVELOPE.md`), save current State.
//! Checkpoint includes `reducer_version`, `commit_index` of last event
//! reduced, and the serialized State. Checkpoints are derived artifacts --
//! deletable, always rebuildable from EventLog.
//!
//! # state_hash
//!
//! `state_hash = BLAKE3(reducer_version_bytes + canonical_serialize(State))`
//!
//! INCLUDE list (all State fields): run_metadata, event_counts_by_type,
//! event_counts_by_tier, tool_summaries, policy_decisions, error_log,
//! clock_skew_events, redaction_log, last_commit_index, tier_a_count,
//! tier_a_drops.
//!
//! EXCLUDE list: nothing. All State fields affect replay correctness.
//!
//! # Invariants enforced
//!
//! - **I2 (Deterministic projection):** State is the input to projection.
//! - **I4 (Testable determinism):** `state_hash` stability across runs.

use crate::event::{CommittedEvent, EventPayload, Tier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Reducer logic version. Included in state_hash so that reducer changes
/// produce visibly different hashes.
pub(crate) const REDUCER_VERSION: &str = "reducer-v0.1";

/// Checkpoint interval from `docs/CAPACITY_ENVELOPE.md`.
pub(crate) const CHECKPOINT_INTERVAL: u64 = 5000;

// ---------------------------------------------------------------------------
// State (M4.1)
// ---------------------------------------------------------------------------

/// Accumulated state from replaying the EventLog.
///
/// All map-like containers are [`BTreeMap`] for deterministic serialization
/// and hashing. This struct is the single input to projection (M5).
///
/// # Determinism
///
/// - No `HashMap` anywhere.
/// - No floats.
/// - All fields are deterministic given the same event sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// Metadata for each run, keyed by `run_id`.
    pub run_metadata: BTreeMap<String, RunInfo>,
    /// Event counts by payload type name (e.g., "RunStart", "ToolCall").
    pub event_counts_by_type: BTreeMap<String, u64>,
    /// Event counts by tier.
    pub event_counts_by_tier: BTreeMap<Tier, u64>,
    /// Tool usage summaries, keyed by tool name.
    pub tool_summaries: BTreeMap<String, ToolSummary>,
    /// Policy (backpressure) transitions in order.
    pub policy_decisions: Vec<PolicyTransition>,
    /// Errors recorded in order.
    pub error_log: Vec<ErrorEntry>,
    /// Clock skew detections in order.
    pub clock_skew_events: Vec<ClockSkewEntry>,
    /// Redactions applied in order.
    pub redaction_log: Vec<RedactionEntry>,
    /// `commit_index` of the last event reduced. 0 if no events.
    pub last_commit_index: u64,
    /// Total Tier A events processed.
    pub tier_a_count: u64,
    /// Tier A drops (should always be 0 in v0.1).
    pub tier_a_drops: u64,
}

impl State {
    /// Create an empty initial state.
    pub fn new() -> Self {
        State {
            run_metadata: BTreeMap::new(),
            event_counts_by_type: BTreeMap::new(),
            event_counts_by_tier: BTreeMap::new(),
            tool_summaries: BTreeMap::new(),
            policy_decisions: Vec::new(),
            error_log: Vec::new(),
            clock_skew_events: Vec::new(),
            redaction_log: Vec::new(),
            last_commit_index: 0,
            tier_a_count: 0,
            tier_a_drops: 0,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Run-level metadata accumulated from RunStart/RunEnd events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunInfo {
    /// Agent identifier from RunStart.
    pub agent: String,
    /// Command args from RunStart, if present.
    pub args: Option<String>,
    /// Whether we have seen a RunEnd for this run.
    pub ended: bool,
    /// Exit code from RunEnd, if available.
    pub exit_code: Option<i32>,
    /// Reason from RunEnd, if available.
    pub reason: Option<String>,
    /// Total events in this run.
    pub event_count: u64,
}

/// Tool usage summary accumulated from ToolCall/ToolResult events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSummary {
    /// Total number of calls to this tool.
    pub call_count: u64,
    /// Total number of results from this tool.
    pub result_count: u64,
    /// Number of successful results.
    pub success_count: u64,
    /// Number of error results.
    pub error_count: u64,
}

impl ToolSummary {
    fn new() -> Self {
        ToolSummary {
            call_count: 0,
            result_count: 0,
            success_count: 0,
            error_count: 0,
        }
    }
}

/// A recorded policy/backpressure transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyTransition {
    /// `commit_index` of the PolicyDecision event.
    pub commit_index: u64,
    /// Ladder level before transition.
    pub from_level: String,
    /// Ladder level after transition.
    pub to_level: String,
    /// Trigger description.
    pub trigger: String,
    /// Queue pressure quantized to millionths (0..=1_000_000) to avoid
    /// f64 nondeterminism. `queue_pressure * 1_000_000` rounded to u64.
    pub queue_pressure_micro: u64,
}

/// A recorded error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEntry {
    /// `commit_index` of the Error event.
    pub commit_index: u64,
    /// Error classification.
    pub kind: String,
    /// Human-readable message.
    pub message: String,
    /// Severity if available.
    pub severity: Option<String>,
}

/// A recorded clock skew detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockSkewEntry {
    /// `commit_index` of the ClockSkewDetected event.
    pub commit_index: u64,
    /// Expected minimum timestamp.
    pub expected_ns: u64,
    /// Actual observed timestamp.
    pub actual_ns: u64,
    /// Backward delta.
    pub delta_ns: u64,
}

/// A recorded redaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionEntry {
    /// `commit_index` of the RedactionApplied event.
    pub commit_index: u64,
    /// Event ID that was redacted.
    pub target_event_id: String,
    /// Field path that was redacted.
    pub field_path: String,
    /// Reason for redaction.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Checkpoint (M4.3)
// ---------------------------------------------------------------------------

/// Versioned checkpoint of reducer state.
///
/// Checkpoints are derived artifacts -- always rebuildable from EventLog.
/// If `reducer_version` doesn't match the current version, discard and
/// replay from scratch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Reducer version that produced this checkpoint.
    pub reducer_version: String,
    /// `commit_index` of the last event reduced into this state.
    pub commit_index: u64,
    /// The accumulated state.
    pub state: State,
}

// ---------------------------------------------------------------------------
// Pure reduce function (M4.2)
// ---------------------------------------------------------------------------

/// Pure reduce function: `(State, CommittedEvent) -> State`.
///
/// No IO, no randomness, no wall clock. Same inputs always produce the
/// same output. Processes all event variants including Generic.
///
/// The `synthesized` flag is metadata -- synthesized events are processed
/// identically to non-synthesized events.
pub fn reduce(state: &State, event: &CommittedEvent) -> State {
    let mut s = state.clone();
    reduce_in_place(&mut s, event);
    s
}

/// In-place reducer variant used by replay-heavy call sites.
///
/// This applies the exact same state transition as [`reduce`] without cloning
/// the full state per event.
pub fn reduce_in_place(s: &mut State, event: &CommittedEvent) {
    // Update last_commit_index.
    s.last_commit_index = event.commit_index;

    // Count by payload type.
    *s.event_counts_by_type
        .entry(event.payload.event_type_name().to_string())
        .or_insert(0) += 1;

    // Count by tier.
    *s.event_counts_by_tier.entry(event.tier).or_insert(0) += 1;

    // Count Tier A.
    if event.tier == Tier::A {
        s.tier_a_count += 1;
    }

    // Per-run event counting.
    let run = s
        .run_metadata
        .entry(event.run_id.clone())
        .or_insert_with(|| RunInfo {
            agent: String::new(),
            args: None,
            ended: false,
            exit_code: None,
            reason: None,
            event_count: 0,
        });
    run.event_count += 1;

    // Dispatch on payload variant.
    match &event.payload {
        EventPayload::RunStart { agent, args } => {
            run.agent = agent.clone();
            run.args = args.clone();
        }
        EventPayload::RunEnd {
            exit_code, reason, ..
        } => {
            run.ended = true;
            run.exit_code = *exit_code;
            run.reason = reason.clone();
        }
        EventPayload::ToolCall { tool, .. } => {
            s.tool_summaries
                .entry(tool.clone())
                .or_insert_with(ToolSummary::new)
                .call_count += 1;
        }
        EventPayload::ToolResult { tool, status, .. } => {
            let summary = s
                .tool_summaries
                .entry(tool.clone())
                .or_insert_with(ToolSummary::new);
            summary.result_count += 1;
            match status.as_deref() {
                Some("success") => summary.success_count += 1,
                Some("error") => summary.error_count += 1,
                _ => {} // unknown or absent status -- counted in result_count only
            }
        }
        EventPayload::PolicyDecision {
            from_level,
            to_level,
            trigger,
            queue_pressure,
        } => {
            // Quantize f64 queue_pressure to millionths for deterministic State.
            // Clamp to [0.0, 1.0], round to avoid IEEE 754 truncation errors.
            let clamped = queue_pressure.clamp(0.0, 1.0);
            let qp_micro = (clamped * 1_000_000.0).round() as u64;
            s.policy_decisions.push(PolicyTransition {
                commit_index: event.commit_index,
                from_level: from_level.clone(),
                to_level: to_level.clone(),
                trigger: trigger.clone(),
                queue_pressure_micro: qp_micro,
            });
        }
        EventPayload::RedactionApplied {
            target_event_id,
            field_path,
            reason,
        } => {
            s.redaction_log.push(RedactionEntry {
                commit_index: event.commit_index,
                target_event_id: target_event_id.clone(),
                field_path: field_path.clone(),
                reason: reason.clone(),
            });
        }
        EventPayload::Error {
            kind,
            message,
            severity,
        } => {
            s.error_log.push(ErrorEntry {
                commit_index: event.commit_index,
                kind: kind.clone(),
                message: message.clone(),
                severity: severity.clone(),
            });
        }
        EventPayload::ClockSkewDetected {
            expected_ns,
            actual_ns,
            delta_ns,
        } => {
            s.clock_skew_events.push(ClockSkewEntry {
                commit_index: event.commit_index,
                expected_ns: *expected_ns,
                actual_ns: *actual_ns,
                delta_ns: *delta_ns,
            });
        }
        EventPayload::Generic { event_type, .. } => {
            // Generic events are counted by type name in event_counts_by_type
            // (already handled above via event_type_name()). Also count by
            // the specific event_type string for finer granularity.
            *s.event_counts_by_type
                .entry(format!("Generic:{event_type}"))
                .or_insert(0) += 1;
        }
    }
}

/// Replay a sequence of committed events from an initial state.
///
/// Returns the final state plus a list of commit_index values where
/// checkpoints should be written (every [`CHECKPOINT_INTERVAL`] events).
pub fn replay(events: &[CommittedEvent]) -> (State, Vec<u64>) {
    replay_from(State::new(), events)
}

/// Replay from a given state (e.g., loaded from a checkpoint).
pub fn replay_from(initial: State, events: &[CommittedEvent]) -> (State, Vec<u64>) {
    let mut state = initial;
    let mut checkpoint_indices = Vec::new();

    for event in events {
        reduce_in_place(&mut state, event);

        // Check if we should checkpoint. Checkpoint at every CHECKPOINT_INTERVAL
        // boundary. commit_index is 0-based, so checkpoint after index 4999, 9999, etc.
        if (event.commit_index + 1) % CHECKPOINT_INTERVAL == 0 {
            checkpoint_indices.push(event.commit_index);
        }
    }

    (state, checkpoint_indices)
}

// ---------------------------------------------------------------------------
// state_hash (M4.4)
// ---------------------------------------------------------------------------

/// Compute the state hash: `BLAKE3(reducer_version + canonical_serialize(State))`.
///
/// # Hash input composition
///
/// 1. `REDUCER_VERSION` as UTF-8 bytes.
/// 2. Deterministic JSON serialization of the entire `State` struct.
///
/// # INCLUDE list
///
/// All fields of [`State`]: `run_metadata`, `event_counts_by_type`,
/// `event_counts_by_tier`, `tool_summaries`, `policy_decisions`,
/// `error_log`, `clock_skew_events`, `redaction_log`, `last_commit_index`,
/// `tier_a_count`, `tier_a_drops`.
///
/// # EXCLUDE list
///
/// Nothing excluded. All State fields affect replay correctness.
pub fn state_hash(state: &State) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(REDUCER_VERSION.as_bytes());
    // Canonical JSON serialization. serde_json serializes struct fields in
    // declaration order, BTreeMap keys in sorted order.
    // State contains only primitive types and BTreeMaps, so serialization
    // should never fail.
    let state_bytes = serde_json::to_vec(state).expect("State serialization should never fail");
    hasher.update(&state_bytes);
    hasher.finalize().to_hex().to_string()
}

// ---------------------------------------------------------------------------
// Checkpoint serialization (M4.3)
// ---------------------------------------------------------------------------

/// Create a checkpoint for the current state.
#[allow(dead_code)] // Used in tests, will be used for checkpoint persistence
pub(crate) fn create_checkpoint(state: &State) -> Checkpoint {
    Checkpoint {
        reducer_version: REDUCER_VERSION.to_string(),
        commit_index: state.last_commit_index,
        state: state.clone(),
    }
}

/// Serialize a checkpoint to JSON bytes.
#[allow(dead_code)] // Used in tests, will be used for checkpoint persistence
pub(crate) fn serialize_checkpoint(checkpoint: &Checkpoint) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(checkpoint)
}

/// Deserialize a checkpoint from JSON bytes.
///
/// Returns `None` if deserialization fails or if the `reducer_version`
/// doesn't match the current version (stale checkpoint).
#[allow(dead_code)] // Used in tests, will be used for checkpoint loading
pub(crate) fn load_checkpoint(data: &[u8]) -> Option<Checkpoint> {
    let checkpoint: Checkpoint = serde_json::from_slice(data).ok()?;
    if checkpoint.reducer_version != REDUCER_VERSION {
        return None;
    }
    Some(checkpoint)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CommittedEvent, EventPayload, ImportEvent, Tier};
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_committed(commit_index: u64, payload: EventPayload) -> CommittedEvent {
        CommittedEvent::commit(
            ImportEvent {
                run_id: "run-1".into(),
                event_id: format!("e-{commit_index}"),
                source_id: "test".into(),
                source_seq: Some(commit_index),
                timestamp_ns: 1_000_000_000 + commit_index * 1_000_000,
                tier: Tier::A,
                payload,
                payload_ref: None,
                synthesized: false,
            },
            commit_index,
        )
    }

    fn make_committed_with_run(
        commit_index: u64,
        run_id: &str,
        payload: EventPayload,
    ) -> CommittedEvent {
        CommittedEvent::commit(
            ImportEvent {
                run_id: run_id.into(),
                event_id: format!("e-{commit_index}"),
                source_id: "test".into(),
                source_seq: Some(commit_index),
                timestamp_ns: 1_000_000_000 + commit_index * 1_000_000,
                tier: Tier::A,
                payload,
                payload_ref: None,
                synthesized: false,
            },
            commit_index,
        )
    }

    fn make_tier_b_committed(commit_index: u64, event_type: &str) -> CommittedEvent {
        CommittedEvent::commit(
            ImportEvent {
                run_id: "run-1".into(),
                event_id: format!("e-{commit_index}"),
                source_id: "test".into(),
                source_seq: Some(commit_index),
                timestamp_ns: 1_000_000_000 + commit_index * 1_000_000,
                tier: Tier::B,
                payload: EventPayload::Generic {
                    event_type: event_type.into(),
                    data: BTreeMap::new(),
                },
                payload_ref: None,
                synthesized: false,
            },
            commit_index,
        )
    }

    // -----------------------------------------------------------------------
    // M4.1: State struct tests
    // -----------------------------------------------------------------------

    #[test]
    fn new_state_is_empty() {
        let s = State::new();
        assert!(s.run_metadata.is_empty());
        assert!(s.event_counts_by_type.is_empty());
        assert!(s.event_counts_by_tier.is_empty());
        assert!(s.tool_summaries.is_empty());
        assert!(s.policy_decisions.is_empty());
        assert!(s.error_log.is_empty());
        assert!(s.clock_skew_events.is_empty());
        assert!(s.redaction_log.is_empty());
        assert_eq!(s.last_commit_index, 0);
        assert_eq!(s.tier_a_count, 0);
        assert_eq!(s.tier_a_drops, 0);
    }

    #[test]
    fn state_default_equals_new() {
        assert_eq!(State::new(), State::default());
    }

    #[test]
    fn state_serialize_roundtrip() {
        let state = State::new();
        let json = serde_json::to_string(&state).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(state, back);
    }

    #[test]
    fn state_uses_btreemap_only() {
        // Compile-time guarantee: all map fields are BTreeMap.
        // This test verifies that serialized keys are sorted.
        let mut state = State::new();
        state.event_counts_by_type.insert("Zebra".into(), 1);
        state.event_counts_by_type.insert("Alpha".into(), 2);
        let json = serde_json::to_string(&state).unwrap();
        let alpha_pos = json.find("\"Alpha\"").unwrap();
        let zebra_pos = json.find("\"Zebra\"").unwrap();
        assert!(
            alpha_pos < zebra_pos,
            "BTreeMap keys must be sorted in JSON"
        );
    }

    // -----------------------------------------------------------------------
    // M4.2: Reduce function tests
    // -----------------------------------------------------------------------

    #[test]
    fn reduce_run_start() {
        let event = make_committed(
            0,
            EventPayload::RunStart {
                agent: "claude-code".into(),
                args: Some("--mode test".into()),
            },
        );
        let state = reduce(&State::new(), &event);
        assert_eq!(state.last_commit_index, 0);
        assert_eq!(state.tier_a_count, 1);
        let run = state.run_metadata.get("run-1").unwrap();
        assert_eq!(run.agent, "claude-code");
        assert_eq!(run.args.as_deref(), Some("--mode test"));
        assert!(!run.ended);
        assert_eq!(run.event_count, 1);
        assert_eq!(state.event_counts_by_type["RunStart"], 1);
        assert_eq!(state.event_counts_by_tier[&Tier::A], 1);
    }

    #[test]
    fn reduce_run_end() {
        let start = make_committed(
            0,
            EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            },
        );
        let end = make_committed(
            1,
            EventPayload::RunEnd {
                exit_code: Some(0),
                reason: Some("done".into()),
            },
        );
        let state = reduce(&reduce(&State::new(), &start), &end);
        let run = state.run_metadata.get("run-1").unwrap();
        assert!(run.ended);
        assert_eq!(run.exit_code, Some(0));
        assert_eq!(run.reason.as_deref(), Some("done"));
        assert_eq!(run.event_count, 2);
    }

    #[test]
    fn reduce_tool_call_and_result() {
        let call = make_committed(
            0,
            EventPayload::ToolCall {
                tool: "Read".into(),
                args: Some("/path".into()),
            },
        );
        let result = make_committed(
            1,
            EventPayload::ToolResult {
                tool: "Read".into(),
                result: Some("content".into()),
                status: Some("success".into()),
            },
        );
        let state = reduce(&reduce(&State::new(), &call), &result);
        let summary = state.tool_summaries.get("Read").unwrap();
        assert_eq!(summary.call_count, 1);
        assert_eq!(summary.result_count, 1);
        assert_eq!(summary.success_count, 1);
        assert_eq!(summary.error_count, 0);
    }

    #[test]
    fn reduce_tool_result_error_status() {
        let result = make_committed(
            0,
            EventPayload::ToolResult {
                tool: "Bash".into(),
                result: Some("command failed".into()),
                status: Some("error".into()),
            },
        );
        let state = reduce(&State::new(), &result);
        let summary = state.tool_summaries.get("Bash").unwrap();
        assert_eq!(summary.result_count, 1);
        assert_eq!(summary.success_count, 0);
        assert_eq!(summary.error_count, 1);
    }

    #[test]
    fn reduce_policy_decision() {
        let event = make_committed(
            0,
            EventPayload::PolicyDecision {
                from_level: "L0".into(),
                to_level: "L1".into(),
                trigger: "queue_pressure".into(),
                queue_pressure: 0.85,
            },
        );
        let state = reduce(&State::new(), &event);
        assert_eq!(state.policy_decisions.len(), 1);
        let pd = &state.policy_decisions[0];
        assert_eq!(pd.from_level, "L0");
        assert_eq!(pd.to_level, "L1");
        assert_eq!(pd.trigger, "queue_pressure");
        assert_eq!(pd.queue_pressure_micro, 850_000);
        assert_eq!(pd.commit_index, 0);
    }

    #[test]
    fn reduce_policy_decision_quantization_edge_cases() {
        // Boundary values and clamping.
        let cases: Vec<(f64, u64)> = vec![
            (0.0, 0),
            (1.0, 1_000_000),
            (0.5, 500_000),
            (0.85, 850_000),
            (0.123456, 123_456),
            (-0.5, 0),        // negative clamped to 0
            (1.5, 1_000_000), // above 1.0 clamped to 1_000_000
        ];
        for (qp, expected_micro) in cases {
            let event = make_committed(
                0,
                EventPayload::PolicyDecision {
                    from_level: "L0".into(),
                    to_level: "L1".into(),
                    trigger: "test".into(),
                    queue_pressure: qp,
                },
            );
            let state = reduce(&State::new(), &event);
            assert_eq!(
                state.policy_decisions[0].queue_pressure_micro, expected_micro,
                "queue_pressure={qp} should quantize to {expected_micro}"
            );
        }
    }

    #[test]
    fn reduce_redaction_applied() {
        let event = make_committed(
            0,
            EventPayload::RedactionApplied {
                target_event_id: "e-5".into(),
                field_path: "payload.args".into(),
                reason: "contains API key".into(),
            },
        );
        let state = reduce(&State::new(), &event);
        assert_eq!(state.redaction_log.len(), 1);
        let r = &state.redaction_log[0];
        assert_eq!(r.target_event_id, "e-5");
        assert_eq!(r.field_path, "payload.args");
        assert_eq!(r.reason, "contains API key");
    }

    #[test]
    fn reduce_error() {
        let event = make_committed(
            0,
            EventPayload::Error {
                kind: "io".into(),
                message: "disk full".into(),
                severity: Some("critical".into()),
            },
        );
        let state = reduce(&State::new(), &event);
        assert_eq!(state.error_log.len(), 1);
        assert_eq!(state.error_log[0].kind, "io");
        assert_eq!(state.error_log[0].severity.as_deref(), Some("critical"));
    }

    #[test]
    fn reduce_clock_skew_detected() {
        let event = make_committed(
            0,
            EventPayload::ClockSkewDetected {
                expected_ns: 2_000_000_000,
                actual_ns: 1_900_000_000,
                delta_ns: 100_000_000,
            },
        );
        let state = reduce(&State::new(), &event);
        assert_eq!(state.clock_skew_events.len(), 1);
        assert_eq!(state.clock_skew_events[0].delta_ns, 100_000_000);
    }

    #[test]
    fn reduce_generic_event() {
        let event = make_tier_b_committed(0, "HeartBeat");
        let state = reduce(&State::new(), &event);
        // Counted by "Generic" (the payload type name)
        assert_eq!(state.event_counts_by_type["Generic"], 1);
        // Also counted by specific "Generic:HeartBeat"
        assert_eq!(state.event_counts_by_type["Generic:HeartBeat"], 1);
        assert_eq!(state.event_counts_by_tier[&Tier::B], 1);
    }

    #[test]
    fn reduce_multiple_runs() {
        let start1 = make_committed_with_run(
            0,
            "run-a",
            EventPayload::RunStart {
                agent: "agent-a".into(),
                args: None,
            },
        );
        let start2 = make_committed_with_run(
            1,
            "run-b",
            EventPayload::RunStart {
                agent: "agent-b".into(),
                args: None,
            },
        );
        let state = reduce(&reduce(&State::new(), &start1), &start2);
        assert_eq!(state.run_metadata.len(), 2);
        assert_eq!(state.run_metadata["run-a"].agent, "agent-a");
        assert_eq!(state.run_metadata["run-b"].agent, "agent-b");
    }

    #[test]
    fn reduce_synthesized_treated_identically() {
        let mut event = make_committed(
            0,
            EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            },
        );
        event.synthesized = true;
        let state = reduce(&State::new(), &event);
        // Synthesized events are processed the same way.
        assert_eq!(state.tier_a_count, 1);
        assert_eq!(state.run_metadata["run-1"].agent, "test");
    }

    // -----------------------------------------------------------------------
    // M4.2: Replay tests
    // -----------------------------------------------------------------------

    #[test]
    fn replay_empty() {
        let (state, checkpoints) = replay(&[]);
        assert_eq!(state, State::new());
        assert!(checkpoints.is_empty());
    }

    #[test]
    fn replay_1000_events_monotonic() {
        let events: Vec<_> = (0..1000)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "Bash".into(),
                        args: Some(format!("cmd-{i}")),
                    },
                )
            })
            .collect();
        let (state, checkpoints) = replay(&events);
        assert_eq!(state.last_commit_index, 999);
        assert_eq!(state.tier_a_count, 1000);
        assert_eq!(state.event_counts_by_type["ToolCall"], 1000);
        assert_eq!(state.tool_summaries["Bash"].call_count, 1000);
        assert!(checkpoints.is_empty()); // 1000 < 5000
    }

    #[test]
    fn replay_matches_clone_based_reduce_path() {
        let events = vec![
            make_committed(
                0,
                EventPayload::RunStart {
                    agent: "agent-a".into(),
                    args: Some("--mode test".into()),
                },
            ),
            make_committed(
                1,
                EventPayload::ToolCall {
                    tool: "Read".into(),
                    args: Some("file.txt".into()),
                },
            ),
            make_committed(
                2,
                EventPayload::ToolResult {
                    tool: "Read".into(),
                    status: Some("success".into()),
                    result: Some("ok".into()),
                },
            ),
            make_committed(
                3,
                EventPayload::PolicyDecision {
                    from_level: "L0".into(),
                    to_level: "L1".into(),
                    trigger: "queue pressure".into(),
                    queue_pressure: 0.551_234,
                },
            ),
            make_committed(
                4,
                EventPayload::Error {
                    kind: "tool".into(),
                    message: "failed".into(),
                    severity: Some("warn".into()),
                },
            ),
            make_committed(
                5,
                EventPayload::RunEnd {
                    exit_code: Some(0),
                    reason: Some("done".into()),
                },
            ),
        ];

        let (replay_state, replay_checkpoints) = replay(&events);

        let mut clone_path_state = State::new();
        for event in &events {
            clone_path_state = reduce(&clone_path_state, event);
        }

        assert_eq!(replay_state, clone_path_state);
        assert_eq!(state_hash(&replay_state), state_hash(&clone_path_state));
        assert!(replay_checkpoints.is_empty());
    }

    // -----------------------------------------------------------------------
    // M4.3: Checkpoint tests
    // -----------------------------------------------------------------------

    #[test]
    fn checkpoint_serialization_roundtrip() {
        let mut state = State::new();
        state.last_commit_index = 4999;
        state.tier_a_count = 5000;
        state.event_counts_by_type.insert("RunStart".into(), 1);
        let checkpoint = create_checkpoint(&state);
        let bytes = serialize_checkpoint(&checkpoint).unwrap();
        let loaded = load_checkpoint(&bytes).unwrap();
        assert_eq!(loaded.reducer_version, REDUCER_VERSION);
        assert_eq!(loaded.commit_index, 4999);
        assert_eq!(loaded.state, state);
    }

    #[test]
    fn checkpoint_version_mismatch_returns_none() {
        let checkpoint = Checkpoint {
            reducer_version: "reducer-v0.0-stale".into(),
            commit_index: 100,
            state: State::new(),
        };
        let bytes = serde_json::to_vec(&checkpoint).unwrap();
        assert!(load_checkpoint(&bytes).is_none());
    }

    #[test]
    fn checkpoint_corrupt_data_returns_none() {
        assert!(load_checkpoint(b"not json").is_none());
        assert!(load_checkpoint(b"{}").is_none());
    }

    #[test]
    fn checkpoint_interval_at_5000() {
        let events: Vec<_> = (0..5001)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "T".into(),
                        args: None,
                    },
                )
            })
            .collect();
        let (state, checkpoints) = replay(&events);
        assert_eq!(state.last_commit_index, 5000);
        // Checkpoint at index 4999 (5000th event).
        assert_eq!(checkpoints, vec![4999]);
    }

    #[test]
    fn checkpoint_interval_at_10000() {
        let events: Vec<_> = (0..10001)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "T".into(),
                        args: None,
                    },
                )
            })
            .collect();
        let (_, checkpoints) = replay(&events);
        assert_eq!(checkpoints, vec![4999, 9999]);
    }

    // -----------------------------------------------------------------------
    // M4.4: state_hash tests
    // -----------------------------------------------------------------------

    #[test]
    fn state_hash_empty_state() {
        let hash = state_hash(&State::new());
        // Should be a 64-character hex string (BLAKE3).
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn state_hash_changes_with_state() {
        let empty_hash = state_hash(&State::new());
        let event = make_committed(
            0,
            EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            },
        );
        let state = reduce(&State::new(), &event);
        let filled_hash = state_hash(&state);
        assert_ne!(empty_hash, filled_hash);
    }

    #[test]
    fn state_hash_stable_across_calls() {
        let event = make_committed(
            0,
            EventPayload::RunStart {
                agent: "test".into(),
                args: None,
            },
        );
        let state = reduce(&State::new(), &event);
        let hash1 = state_hash(&state);
        let hash2 = state_hash(&state);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn state_hash_includes_reducer_version() {
        // Verify that state_hash would change if REDUCER_VERSION changed
        // by manually computing with a different version prefix.
        let state = State::new();
        let normal_hash = state_hash(&state);

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"reducer-v999.0");
        hasher.update(&serde_json::to_vec(&state).unwrap());
        let different_version_hash = hasher.finalize().to_hex().to_string();

        assert_ne!(normal_hash, different_version_hash);
    }

    // -----------------------------------------------------------------------
    // M4.5: Checkpoint rebuild equivalence test
    // -----------------------------------------------------------------------

    #[test]
    fn checkpoint_rebuild_equivalence() {
        // Create an EventLog with > 5000 events.
        let events: Vec<_> = (0..6000)
            .map(|i| match i % 5 {
                0 => make_committed(
                    i,
                    EventPayload::RunStart {
                        agent: format!("agent-{}", i / 100),
                        args: None,
                    },
                ),
                1 => make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "Read".into(),
                        args: Some(format!("/path/{i}")),
                    },
                ),
                2 => make_committed(
                    i,
                    EventPayload::ToolResult {
                        tool: "Read".into(),
                        result: Some(format!("content-{i}")),
                        status: Some("success".into()),
                    },
                ),
                3 => make_committed(
                    i,
                    EventPayload::Error {
                        kind: "test".into(),
                        message: format!("err-{i}"),
                        severity: None,
                    },
                ),
                _ => make_tier_b_committed(i, "Metric"),
            })
            .collect();

        // Full replay.
        let (state_full, _) = replay(&events);

        // Checkpoint replay: reduce first 5000, checkpoint, then resume.
        let (state_at_checkpoint, _) = replay(&events[..5000]);
        let checkpoint = create_checkpoint(&state_at_checkpoint);
        let checkpoint_bytes = serialize_checkpoint(&checkpoint).unwrap();
        let loaded = load_checkpoint(&checkpoint_bytes).unwrap();

        // Resume from checkpoint.
        let (state_checkpoint, _) = replay_from(loaded.state, &events[5000..]);

        // Assert equivalence.
        assert_eq!(state_full, state_checkpoint);
        assert_eq!(state_hash(&state_full), state_hash(&state_checkpoint));
    }

    #[test]
    fn checkpoint_rebuild_at_exact_boundary() {
        // Exactly 5000 events -- checkpoint at 4999, no remainder.
        let events: Vec<_> = (0..5000)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "T".into(),
                        args: None,
                    },
                )
            })
            .collect();

        let (state_full, checkpoints) = replay(&events);
        assert_eq!(checkpoints, vec![4999]);

        let (state_at_cp, _) = replay(&events[..5000]);
        let checkpoint = create_checkpoint(&state_at_cp);
        let loaded = load_checkpoint(&serialize_checkpoint(&checkpoint).unwrap()).unwrap();
        let (state_from_cp, _) = replay_from(loaded.state, &[]);

        assert_eq!(state_full, state_from_cp);
        assert_eq!(state_hash(&state_full), state_hash(&state_from_cp));
    }

    #[test]
    fn checkpoint_rebuild_at_5001() {
        // 5001 events: checkpoint at 4999, 1 remaining event.
        let events: Vec<_> = (0..5001)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "T".into(),
                        args: None,
                    },
                )
            })
            .collect();

        let (state_full, _) = replay(&events);

        let (state_at_cp, _) = replay(&events[..5000]);
        let loaded =
            load_checkpoint(&serialize_checkpoint(&create_checkpoint(&state_at_cp)).unwrap())
                .unwrap();
        let (state_from_cp, _) = replay_from(loaded.state, &events[5000..]);

        assert_eq!(state_full, state_from_cp);
        assert_eq!(state_hash(&state_full), state_hash(&state_from_cp));
    }

    #[test]
    fn checkpoint_rebuild_at_10000() {
        // 10001 events: checkpoints at 4999 and 9999.
        let events: Vec<_> = (0..10001)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: "T".into(),
                        args: None,
                    },
                )
            })
            .collect();

        let (state_full, checkpoints) = replay(&events);
        assert_eq!(checkpoints, vec![4999, 9999]);

        // Resume from second checkpoint (at 9999, i.e., first 10000 events).
        let (state_at_cp2, _) = replay(&events[..10000]);
        let loaded =
            load_checkpoint(&serialize_checkpoint(&create_checkpoint(&state_at_cp2)).unwrap())
                .unwrap();
        let (state_from_cp2, _) = replay_from(loaded.state, &events[10000..]);

        assert_eq!(state_full, state_from_cp2);
        assert_eq!(state_hash(&state_full), state_hash(&state_from_cp2));
    }

    // -----------------------------------------------------------------------
    // M4.6: Determinism test (10 runs)
    // -----------------------------------------------------------------------

    #[test]
    fn determinism_10_runs() {
        // Create a deterministic event sequence covering all payload variants.
        let events: Vec<_> = (0..100)
            .map(|i| match i % 9 {
                0 => make_committed_with_run(
                    i,
                    &format!("run-{}", i / 9),
                    EventPayload::RunStart {
                        agent: format!("agent-{}", i / 9),
                        args: Some(format!("--arg {i}")),
                    },
                ),
                1 => make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: format!("tool-{}", i % 3),
                        args: Some(format!("args-{i}")),
                    },
                ),
                2 => make_committed(
                    i,
                    EventPayload::ToolResult {
                        tool: format!("tool-{}", i % 3),
                        result: Some(format!("result-{i}")),
                        status: Some("success".into()),
                    },
                ),
                3 => make_committed(
                    i,
                    EventPayload::PolicyDecision {
                        from_level: "L0".into(),
                        to_level: "L1".into(),
                        trigger: format!("trigger-{i}"),
                        queue_pressure: 0.85,
                    },
                ),
                4 => make_committed(
                    i,
                    EventPayload::RedactionApplied {
                        target_event_id: format!("e-{}", i - 1),
                        field_path: "payload.args".into(),
                        reason: "secret".into(),
                    },
                ),
                5 => make_committed(
                    i,
                    EventPayload::Error {
                        kind: "test".into(),
                        message: format!("error-{i}"),
                        severity: Some("warning".into()),
                    },
                ),
                6 => make_committed(
                    i,
                    EventPayload::ClockSkewDetected {
                        expected_ns: 2_000_000_000,
                        actual_ns: 1_900_000_000,
                        delta_ns: 100_000_000,
                    },
                ),
                7 => make_tier_b_committed(i, &format!("Metric-{}", i % 3)),
                _ => make_committed_with_run(
                    i,
                    &format!("run-{}", i / 9),
                    EventPayload::RunEnd {
                        exit_code: Some(0),
                        reason: Some("done".into()),
                    },
                ),
            })
            .collect();

        // Run 10 times, collect hashes.
        let mut hashes = Vec::new();
        for _ in 0..10 {
            let (state, _) = replay(&events);
            hashes.push(state_hash(&state));
        }

        // All 10 hashes must be identical.
        for (i, hash) in hashes.iter().enumerate() {
            assert_eq!(
                hash, &hashes[0],
                "Run {i} produced different state_hash: {} vs {}",
                hash, hashes[0]
            );
        }
    }

    #[test]
    fn determinism_10_runs_large() {
        // 5500 events to cross the checkpoint boundary.
        let events: Vec<_> = (0..5500)
            .map(|i| {
                make_committed(
                    i,
                    EventPayload::ToolCall {
                        tool: format!("tool-{}", i % 10),
                        args: Some(format!("arg-{i}")),
                    },
                )
            })
            .collect();

        let mut hashes = Vec::new();
        for _ in 0..10 {
            let (state, _) = replay(&events);
            hashes.push(state_hash(&state));
        }

        for (i, hash) in hashes.iter().enumerate() {
            assert_eq!(
                hash, &hashes[0],
                "Large run {i} produced different state_hash"
            );
        }
    }
}
